import { useState } from "react";
import {
  Alert,
  Button,
  Card,
  PasswordInput,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useMutation } from "@tanstack/react-query";
import { client, result, type Session } from "./api";

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
    <div className="full-center">
      <Card withBorder padding="xl" w="min(440px, 95vw)">
        <Title order={2}>Join your team</Title>
        <Text c="dimmed" size="sm" mt="sm" mb="lg">
          Choose your local password to accept this private invitation.
        </Text>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            enroll.mutate();
          }}
        >
          <Stack>
            <PasswordInput
              label="Choose a password"
              autoComplete="new-password"
              minLength={8}
              maxLength={4096}
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
            {enroll.error && (
              <Alert color="red" title="Invitation not accepted">
                {enroll.error.message}
              </Alert>
            )}
            <Button type="submit" loading={enroll.isPending}>
              Accept invitation
            </Button>
            <Button variant="subtle" onClick={cancel}>
              Back to sign in
            </Button>
          </Stack>
        </form>
      </Card>
    </div>
  );
}
