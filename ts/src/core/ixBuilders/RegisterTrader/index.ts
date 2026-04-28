export {
  getRegisterTraderInstructionCodec,
  getRegisterTraderInstructionDecoder,
  getRegisterTraderInstructionEncoder,
  getRegisterTraderParamsCodec,
  getRegisterTraderParamsDecoder,
  getRegisterTraderParamsEncoder,
} from "./codec";
export type { RegisterTraderParamsData } from "./codec";
export { buildRegisterTraderIx } from "./ix";
export type {
  RegisterTraderAccounts,
  RegisterTraderIx,
  RegisterTraderParams,
} from "./types";
