import { CombinedGraphQLErrors } from "@apollo/client/errors";

export function extractGqlErrorMessage(error: unknown, fallback: string): string {
  if (CombinedGraphQLErrors.is(error)) {
    return error.errors[0]?.message ?? fallback;
  }
  return fallback;
}

export function extractGqlFieldErrors(
  error: unknown,
): Record<string, string[]> | null {
  if (CombinedGraphQLErrors.is(error)) {
    const extensions = error.errors[0]?.extensions as
      | Record<string, unknown>
      | undefined;
    if (extensions?.field_errors) {
      return extensions.field_errors as Record<string, string[]>;
    }
  }
  return null;
}
