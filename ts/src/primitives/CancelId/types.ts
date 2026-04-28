import type { FIFOOrderId } from "../FIFOOrderId";

type NodePointer = number; // u32

export interface CancelId {
  nodePointer: NodePointer | null;
  orderId: FIFOOrderId;
}
