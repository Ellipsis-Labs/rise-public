import z from "zod";

export interface ValidateInviteRequest {
  code: string;
  wallet_address: string;
  transaction_signature?: string;
}

export const ValidateInviteRequestSchema: z.ZodType<ValidateInviteRequest> =
  z.object({
    code: z.string(),
    wallet_address: z.string(),
    transaction_signature: z.string().optional(),
  });

export interface ValidateInviteResponse {
  success: boolean;
  message: string;
  whitelisted: boolean;
}

export const ValidateInviteResponseSchema: z.ZodType<ValidateInviteResponse> =
  z.object({
    success: z.boolean(),
    message: z.string(),
    whitelisted: z.boolean(),
  });

export interface CheckWalletResponse {
  whitelisted: boolean;
  whitelisted_at?: string | null;
  invite_code_used?: string | null;
}

export const CheckWalletResponseSchema: z.ZodType<CheckWalletResponse> =
  z.object({
    whitelisted: z.boolean(),
    whitelisted_at: z.string().nullable().optional(),
    invite_code_used: z.string().nullable().optional(),
  });

export interface ActivateInviteRequest {
  authority: string;
  code: string;
}

export const ActivateInviteRequestSchema: z.ZodType<ActivateInviteRequest> =
  z.object({
    authority: z.string(),
    code: z.string(),
  });

export interface ActivateReferralRequest {
  authority: string;
  referral_code: string;
}

export const ActivateReferralRequestSchema: z.ZodType<ActivateReferralRequest> =
  z.object({
    authority: z.string(),
    referral_code: z.string(),
  });

export interface ActivateReferralTxRequest {
  referral_code: string;
  trader_authority: string;
  trader_pda_index?: number;
  trader_subaccount_index?: number;
  recent_blockhash: string;
  transaction: string;
}

export const ActivateReferralTxRequestSchema: z.ZodType<ActivateReferralTxRequest> =
  z.object({
    referral_code: z.string(),
    trader_authority: z.string(),
    trader_pda_index: z.number().int().min(0).max(255).optional(),
    trader_subaccount_index: z.number().int().min(0).max(255).optional(),
    recent_blockhash: z.string(),
    transaction: z.string(),
  });

export interface ActivateInviteResponse {
  trader_pda: string;
}

export const ActivateInviteResponseSchema: z.ZodType<ActivateInviteResponse> =
  z.object({
    trader_pda: z.string(),
  });

export interface ActivateReferralTxResponse {
  signature?: string;
  trader_pda: string;
  referral_code: string;
  status: ActivateReferralTxStatus;
}

export const ACTIVATE_REFERRAL_TX_STATUSES = [
  "activated",
  "submitted",
  "already_activated",
] as const;

export type ActivateReferralTxStatus =
  (typeof ACTIVATE_REFERRAL_TX_STATUSES)[number];

export const ActivateReferralTxResponseSchema: z.ZodType<ActivateReferralTxResponse> =
  z.object({
    signature: z.string().optional(),
    trader_pda: z.string(),
    referral_code: z.string(),
    status: z.enum(ACTIVATE_REFERRAL_TX_STATUSES),
  });

export interface ReferralActivationPermissionResponse {
  trader_onboarder: string;
  risk_authority: string;
  permission_account: string;
}

export const ReferralActivationPermissionResponseSchema: z.ZodType<ReferralActivationPermissionResponse> =
  z.object({
    trader_onboarder: z.string(),
    risk_authority: z.string(),
    permission_account: z.string(),
  });
