import type { HttpTransport } from "@/http/transport";
import { get, post } from "@/http/transport";
import type {
  ActivateInviteRequest,
  ActivateInviteResponse,
  ActivateInviteWithReferralRequest,
  CheckWalletResponse,
  ValidateInviteRequest,
  ValidateInviteResponse,
} from "./types";
import {
  ActivateInviteRequestSchema,
  ActivateInviteResponseSchema,
  ActivateInviteWithReferralRequestSchema,
  CheckWalletResponseSchema,
  ValidateInviteRequestSchema,
  ValidateInviteResponseSchema,
} from "./types";

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

  async activateInviteWithReferral(
    request: ActivateInviteWithReferralRequest
  ): Promise<ActivateInviteResponse> {
    const payload = ActivateInviteWithReferralRequestSchema.parse(request);
    return post(
      this.http,
      "/v1/invite/activate-with-referral",
      ActivateInviteResponseSchema,
      payload
    );
  }
}
