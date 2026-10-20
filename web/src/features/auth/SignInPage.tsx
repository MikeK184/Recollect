import { useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowRight, Building2 } from "lucide-react";
import { client, result } from "../../api";
import { ErrorState as Failure } from "../../components/AsyncState";
import { replaceSession } from "../../app/session";
import {
  Alert,
  Badge,
  Button,
  Field,
  Input,
  PasswordInput,
} from "../../components/ui";
import { AuthHeading, AuthLayout } from "./AuthLayout";

// Sign in (workspace category). Rendered by App when no session exists.
export function SignInPage({ ended }: { ended: boolean }) {
  const options = useQuery({
    queryKey: ["auth-options"],
    queryFn: async () => result(await client.GET("/api/auth/options")),
  });
  const organizationError = new URLSearchParams(window.location.search).has(
    "auth_error",
  );
  const cache = useQueryClient();
  const [username, setUsername] = useState("owner");
  const [password, setPassword] = useState("");
  const login = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/auth/login", { body: { username, password } }),
      ),
    onSuccess: async (data) => {
      await replaceSession(cache, data);
      setPassword("");
    },
  });
  return (
    <AuthLayout labelledBy="signin-title">
      <AuthHeading
        id="signin-title"
        badge={<Badge variant="brand">Local workspace</Badge>}
        title="Welcome back"
        description="Sign in to your Recollect installation."
      />
      {ended && (
        <Alert variant="warning" role="status">
          Your session ended — sign in again
        </Alert>
      )}
      {organizationError && (
        <Alert variant="destructive">
          Organization sign-in was not completed. Check your enrolled identity
          or retry.
        </Alert>
      )}
      <form
        className="flex flex-col gap-6"
        onSubmit={(event: FormEvent) => {
          event.preventDefault();
          login.mutate();
        }}
      >
        <div className="flex flex-col gap-4">
          <Field label="Username">
            <Input
              size="lg"
              name="username"
              autoComplete="username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
            />
          </Field>
          <Field label="Password">
            <PasswordInput
              size="lg"
              name="password"
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </Field>
        </div>
        <Failure error={login.error} />
        <Button
          type="submit"
          size="lg"
          className="w-full"
          loading={login.isPending}
        >
          Sign in
          <ArrowRight aria-hidden="true" />
        </Button>
      </form>
      {options.data?.oidc_configured && (
        <Button asChild variant="secondary" size="lg" className="-mt-2 w-full">
          <a
            href="/api/auth/oidc/start"
            onClick={() => {
              const code = new URLSearchParams(window.location.search).get(
                "code",
              );
              if (
                ["/devices", "/agents"].includes(window.location.pathname) &&
                code
              ) {
                try {
                  sessionStorage.setItem(
                    "recollect-pairing-return",
                    JSON.stringify({ code, until: Date.now() + 300_000 }),
                  );
                } catch {
                  /* The host CLI still displays the verification link. */
                }
              }
            }}
          >
            <Building2 aria-hidden="true" />
            Sign in with your organization
          </a>
        </Button>
      )}
      <p className="m-0 text-caption text-muted-foreground">
        Use your local account or the owner credentials from setup.
      </p>
    </AuthLayout>
  );
}
