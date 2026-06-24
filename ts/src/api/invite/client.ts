import { PhoenixHttpError } from "@/errors";
import type { HttpTransport } from "@/http/transport";
import { get, post } from "@/http/transport";
import { ExchangeSnapshotViewSchema } from "../exchange";
import {
  buildActivateReferralTxRequest,
  referralActivationExchangeAccountsFromSnapshot,
  type BuildActivateReferralTxRequestResult,
  type BuildActivateReferralTxRequestWithClientParams,
} from "./referralActivationTx";
import type {
  ActivateInviteRequest,
  ActivateInviteResponse,
  ActivateReferralRequest,
  ActivateReferralTxRequest,
  ActivateReferralTxResponse,
  CheckWalletResponse,
  ReferralActivationPermissionResponse,
  ValidateInviteRequest,
  ValidateInviteResponse,
} from "./types";
import {
  ActivateInviteRequestSchema,
  ActivateInviteResponseSchema,
  ActivateReferralRequestSchema,
  ActivateReferralTxRequestSchema,
  ActivateReferralTxResponseSchema,
  CheckWalletResponseSchema,
  ReferralActivationPermissionResponseSchema,
  ValidateInviteRequestSchema,
  ValidateInviteResponseSchema,
} from "./types";

const REFERRAL_ACTIVATION_AUTH_MESSAGE =
  "Referral activation requires an authenticated user session for the authority wallet. " +
  "Create the Rise client with auth enabled and sign in as that wallet owner before calling /v1/referral/activate.";

function withReferralActivationAuthContext(error: unknown): never {
  if (
    error instanceof PhoenixHttpError &&
    (error.status === 401 ||
      error.code === "missing_access_token" ||
      error.code === "invalid_access_token" ||
      error.code === "access_token_expired" ||
      error.code === "session_missing" ||
      error.code === "no_auth_session" ||
      error.code === "user_only")
  ) {
    throw new PhoenixHttpError(
      error.status,
      `${REFERRAL_ACTIVATION_AUTH_MESSAGE}\n${error.message}`,
      error.code,
      error.retryAfterSeconds,
      error.body
    );
  }

  throw error;
}

export class V1InviteClient {
  constructor(private readonly http: HttpTransport) {}

  async validateInvite(
    request: ValidateInviteRequest
  ): Promise<ValidateInviteResponse> {
    const payload = ValidateInviteRequestSchema.parse(request);
    return post(
      this.http,
      "/v1/invite/validate",
      ValidateInviteResponseSchema,
      payload
    );
  }

  async checkWallet(wallet: string): Promise<CheckWalletResponse> {
    return get(
      this.http,
      `/v1/invite/check/${wallet}`,
      CheckWalletResponseSchema
    );
  }

  async activateInvite(
    request: ActivateInviteRequest
  ): Promise<ActivateInviteResponse> {
    const payload = ActivateInviteRequestSchema.parse(request);
    return post(
      this.http,
      "/v1/invite/activate",
      ActivateInviteResponseSchema,
      payload
    );
  }

  async activateReferral(
    request: ActivateReferralRequest
  ): Promise<ActivateInviteResponse> {
    const payload = ActivateReferralRequestSchema.parse(request);
    try {
      return await post(
        this.http,
        "/v1/referral/activate",
        ActivateInviteResponseSchema,
        payload,
        { auth: "required" }
      );
    } catch (error) {
      withReferralActivationAuthContext(error);
    }
  }

  async getReferralActivationPermission(): Promise<ReferralActivationPermissionResponse> {
    return get(
      this.http,
      "/v1/referral/activation-permission",
      ReferralActivationPermissionResponseSchema
    );
  }

  async activateReferralTx(
    request: ActivateReferralTxRequest
  ): Promise<ActivateReferralTxResponse> {
    const payload = ActivateReferralTxRequestSchema.parse(request);
    return post(
      this.http,
      "/v1/referral/activate-tx",
      ActivateReferralTxResponseSchema,
      payload
    );
  }

  async buildActivateReferralTxRequest(
    params: BuildActivateReferralTxRequestWithClientParams
  ): Promise<BuildActivateReferralTxRequestResult> {
    const [permission, snapshot] = await Promise.all([
      params.permission ?? this.getReferralActivationPermission(),
      params.exchangeAccounts
        ? undefined
        : get(this.http, "/v1/exchange/snapshot", ExchangeSnapshotViewSchema),
    ]);

    return buildActivateReferralTxRequest({
      ...params,
      permission,
      exchangeAccounts:
        params.exchangeAccounts ??
        (await referralActivationExchangeAccountsFromSnapshot(
          snapshot!.exchange
        )),
    });
  }
}
