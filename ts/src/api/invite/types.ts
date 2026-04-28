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

export interface ActivateInviteWithReferralRequest {
  authority: string;
  referral_code: string;
}

export const ActivateInviteWithReferralRequestSchema: z.ZodType<ActivateInviteWithReferralRequest> =
  z.object({
    authority: z.string(),
    referral_code: z.string(),
  });

export interface ActivateInviteResponse {
  trader_pda: string;
}

export const ActivateInviteResponseSchema: z.ZodType<ActivateInviteResponse> =
  z.object({
    trader_pda: z.string(),
  });
