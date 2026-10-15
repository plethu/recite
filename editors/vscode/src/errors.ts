export function errorMessage(error: unknown): string {
  if (error && typeof error === "object" && "message" in error && error.message != null) {
    return String(error.message);
  }
  return String(error);
}
