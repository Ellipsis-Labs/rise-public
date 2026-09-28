export {
  getSetOnboarderSignerInstructionCodec,
  getSetOnboarderSignerInstructionDecoder,
  getSetOnboarderSignerInstructionEncoder,
  getSetOnboarderSignerParamsCodec,
  getSetOnboarderSignerParamsDecoder,
  getSetOnboarderSignerParamsEncoder,
} from "./codec";
export type { SetOnboarderSignerParamsData } from "./codec";
export { buildSetOnboarderSignerIx } from "./ix";
export type {
  SetOnboarderSignerAccounts,
  SetOnboarderSignerIx,
  SetOnboarderSignerParams,
} from "./types";
