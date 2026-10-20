import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { ArrowLeft } from "lucide-react";
import { client, result, type Session } from "./api";
import { Alert, Badge, Button, Field, PasswordInput } from "./components/ui";
import { AuthHeading, AuthLayout } from "./features/auth/AuthLayout";

export const initialInvitation = (() => {
  const token =
    new URLSearchParams(window.location.hash.slice(1)).get("invite") ?? "";
  if (token)
    window.history.replaceState(
      null,
      "",
      window.location.pathname + window.location.search,
    );
  return token;
})();
export function Enrollment({
  token,
  complete,
  cancel,
}: {
  token: string;
  complete: (data: Session) => Promise<void>;
  cancel: () => void;
}) {
  const [password, setPassword] = useState("");
  const enroll = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/auth/enroll", { body: { token, password } }),
      ),
    onSuccess: async (data) => {
      setPassword("");
      await complete(data);
    },
  });
  return (
    <AuthLayout labelledBy="enroll-title">
      <AuthHeading
        id="enroll-title"
        badge={<Badge variant="brand">Private invitation</Badge>}
        title="Join your team"
        description="Choose your local password to accept this private invitation."
      />
      <form
        className="flex flex-col gap-6"
        onSubmit={(e) => {
          e.preventDefault();
          enroll.mutate();
        }}
      >
        <Field label="Choose a password" description="At least 8 characters.">
          <PasswordInput
            size="lg"
            autoComplete="new-password"
            minLength={8}
            maxLength={4096}
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </Field>
        {enroll.error && (
          <Alert variant="destructive" title="Invitation not accepted">
            {enroll.error.message}
          </Alert>
        )}
        <div className="flex flex-col gap-2">
          <Button
            type="submit"
            size="lg"
            className="w-full"
            loading={enroll.isPending}
          >
            Accept invitation
          </Button>
          <Button variant="ghost" size="lg" className="w-full" onClick={cancel}>
            <ArrowLeft aria-hidden="true" />
            Back to sign in
          </Button>
        </div>
      </form>
    </AuthLayout>
  );
}
