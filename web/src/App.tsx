import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Enrollment, initialInvitation } from "./Enrollment";
import { onUnauthorized } from "./api";
import { checkSession, revalidateSession } from "./sessionCheck";
import { ErrorState as Failure } from "./components/AsyncState";
import { Brand } from "./components/Brand";
import { Spinner } from "./components/ui/spinner";
import { AppShell } from "./app/AppShell";
import { replaceSession } from "./app/session";
import { SignInPage } from "./features/auth/SignInPage";
import { usePollInterval } from "./usePollInterval";

export function App() {
  const pollSlow = usePollInterval("slow");
  const cache = useQueryClient();
  const navigate = useNavigate();
  const [invitation, setInvitation] = useState(initialInvitation);
  const [sessionEnded, setSessionEnded] = useState(false);
  const check = useMemo(
    () => checkSession(cache, () => setSessionEnded(true)),
    [cache],
  );
  useEffect(() => {
    const readInvite = () => {
      const token = new URLSearchParams(window.location.hash.slice(1)).get(
        "invite",
      );
      if (token) {
        setInvitation(token);
        window.history.replaceState(
          null,
          "",
          window.location.pathname + window.location.search,
        );
      }
    };
    window.addEventListener("hashchange", readInvite);
    return () => window.removeEventListener("hashchange", readInvite);
  }, []);
  useEffect(() => {
    onUnauthorized(() => {
      if (cache.getQueryData(["session"])) void revalidateSession(cache, check);
    });
    return () => onUnauthorized(null);
  }, [cache, check]);
  const auth = useQuery({
    queryKey: ["session"],
    queryFn: check,
    refetchInterval: pollSlow,
  });
  useEffect(() => {
    if (auth.data) setSessionEnded(false);
  }, [auth.data]);
  useEffect(() => {
    if (!auth.data) return;
    try {
      const saved = sessionStorage.getItem("recollect-pairing-return");
      if (!saved) return;
      sessionStorage.removeItem("recollect-pairing-return");
      const pending = JSON.parse(saved);
      if (/^[a-f0-9]{8}$/i.test(pending.code) && pending.until > Date.now()) {
        void navigate({ to: "/devices", search: { code: pending.code } });
      }
    } catch {
      /* A browser with storage disabled keeps the current local URL. */
    }
  }, [auth.data, navigate]);
  if (invitation)
    return (
      <Enrollment
        token={invitation}
        cancel={() => setInvitation("")}
        complete={async (data) => {
          await replaceSession(cache, data);
          setInvitation("");
        }}
      />
    );
  if (auth.isPending)
    return (
      <div className="grid h-full place-items-center bg-background">
        <div className="grid animate-enter justify-items-center gap-4">
          <Brand />
          <Spinner label="Opening Recollect…" />
          <p className="m-0 text-label text-muted-foreground">
            Opening Recollect…
          </p>
        </div>
      </div>
    );
  if (auth.error)
    return (
      <div className="grid h-full place-items-center bg-background p-6">
        <div className="w-full max-w-[480px]">
          <Failure error={auth.error} retry={() => void auth.refetch()} />
        </div>
      </div>
    );
  if (!auth.data) return <SignInPage ended={sessionEnded} />;
  return <AppShell user={auth.data} />;
}
