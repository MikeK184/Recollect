import createClient from "openapi-fetch";
import type { paths, components } from "./api-schema";

export type Brain = components["schemas"]["Brain"];
export type Session = components["schemas"]["SessionInfo"];
export type Audit = components["schemas"]["AuditEvent"];
export type Status = components["schemas"]["ServiceStatus"];

let csrf = "";
export const setCsrf = (value: string) => {
  csrf = value;
};
export const client = createClient<paths>({
  baseUrl: "",
  credentials: "same-origin",
});
let unauthorized: (() => void) | null = null;
// A 401 outside /api/auth/ means the caller's session or device credential was
// not accepted for that one request; it is not proof the session ended. The app
// registers the revalidation (a /api/auth/me check) that decides.
export const onUnauthorized = (handler: (() => void) | null) => {
  unauthorized = handler;
};
client.use({
  onRequest({ request }) {
    if (csrf) request.headers.set("X-CSRF-Token", csrf);
    return request;
  },
  onResponse({ request, response }) {
    // Sign-in and session reads report their own 401. The CSRF token stays
    // until the revalidation confirms the session is gone (see session()).
    if (
      response.status === 401 &&
      !new URL(request.url).pathname.startsWith("/api/auth/")
    ) {
      unauthorized?.();
    }
    return response;
  },
});

export class RequestError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}
export function result<T>(response: {
  data?: T;
  error?: unknown;
  response: Response;
}): T {
  if (!response.response.ok) {
    const error = response.error as { message?: string } | undefined;
    throw new RequestError(
      response.response.status,
      error?.message ?? "The request failed. Please retry.",
    );
  }
  return response.data as T;
}
export const session = async () => {
  const response = await client.GET("/api/auth/me");
  if (response.response.status === 401) {
    setCsrf("");
    return null;
  }
  const value = result(response);
  setCsrf(value.csrf_token);
  return value;
};
