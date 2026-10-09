import { PhoenixAuthError } from "@/errors";

export const loadServiceAccountFileReader = async (): Promise<never> => {
  throw new PhoenixAuthError(
    "Service-account credential files are unavailable in React Native",
    "service_account_credential_file_unavailable"
  );
};
