import type { HttpTransport } from "@/http/transport";
import { get, post } from "@/http/transport";
import type { ParamValue } from "@/http/transport";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import { AccountRole, address } from "@solana/kit";
import type { Authority, TraderAddress } from "@/primitives";
import type { PhoenixFlightClientConfig } from "@/flight/client";
import { resolvePhoenixFlightFeeCollectorTraderAddress } from "@/flight/client";
import type {
  ApiInstructionResponse,
  CancelConditionalOrderRequest,
  CancelStopLossOrderRequest,
  OrderHistoryRequest,
  OrderHistoryResponse,
  OrderHistoryV2Request,
  OrderHistoryV2Response,
  PlaceIsolatedOrderEnhancedInstructionsResponse,
  PlaceIsolatedLimitOrderRequest,
  PlaceIsolatedLimitOrderWithConditionalsRequest,
  PlaceIsolatedMarketOrderRequest,
  PlaceAttachedConditionalOrderRequest,
  PlacePositionConditionalOrderRequest,
  PlaceStopLossOrderRequest,
} from "./types";
import {
  ApiInstructionResponseSchema,
  CancelConditionalOrderRequestSchema,
  CancelStopLossOrderRequestSchema,
  OrderHistoryResponseSchema,
  OrderHistoryV2ResponseSchema,
  PlaceIsolatedOrderEnhancedResponseSchema,
  PlaceIsolatedLimitOrderRequestSchema,
  PlaceIsolatedLimitOrderWithConditionalsRequestSchema,
  PlaceIsolatedMarketOrderRequestSchema,
  PlaceAttachedConditionalOrderRequestSchema,
  PlacePositionConditionalOrderRequestSchema,
  PlaceStopLossOrderRequestSchema,
} from "./types";

const buildOrderHistoryQuery = (
  request?: OrderHistoryRequest
): Record<string, ParamValue> | undefined => {
  if (!request) return undefined;

  const params: Record<string, ParamValue> = {};

  if (request.traderPdaIndex !== undefined)
    params.traderPdaIndex = request.traderPdaIndex;
  if (request.marketSymbol) params.marketSymbol = request.marketSymbol;
  if (request.limit !== undefined) params.limit = request.limit;
  if (request.cursor) params.cursor = request.cursor;
  if (request.privyId) params.privyId = request.privyId;
  if (request.orderStatus) params.orderStatus = request.orderStatus;

  return Object.keys(params).length > 0 ? params : undefined;
};

const buildOrderHistoryV2Query = (
  request?: OrderHistoryV2Request
): Record<string, ParamValue> | undefined => {
  if (!request) return undefined;

  const params: Record<string, ParamValue> = {};

  if (request.marketSymbol) params.market_symbol = request.marketSymbol;
  if (request.limit !== undefined) params.limit = request.limit;
  if (request.cursor) params.cursor = request.cursor;
  if (request.startTime) params.start_time = request.startTime.toISOString();
  if (request.endTime) params.end_time = request.endTime.toISOString();

  return Object.keys(params).length > 0 ? params : undefined;
};

const toInstruction = (
  apiInstruction: ApiInstructionResponse
): InstructionsWithAccountsAndData => ({
  programAddress: address(apiInstruction.programId),
  accounts: apiInstruction.keys.map((account) => ({
    address: address(account.pubkey),
    role: account.isSigner
      ? account.isWritable
        ? AccountRole.WRITABLE_SIGNER
        : AccountRole.READONLY_SIGNER
      : account.isWritable
        ? AccountRole.WRITABLE
        : AccountRole.READONLY,
  })),
  data: Uint8Array.from(apiInstruction.data),
});

interface V1OrdersClientConfig {
  defaultFlight?: {
    config: PhoenixFlightClientConfig;
    resolveFeeCollectorTraderAddress: (
      builderAuthority?: Authority
    ) => Promise<TraderAddress>;
  };
}

const resolveDefaultFlightFeeCollectorTrader = async (
  defaultFlight: NonNullable<V1OrdersClientConfig["defaultFlight"]>,
  requestFlightBuilderAuthority?: Authority
): Promise<TraderAddress> => {
  if (
    requestFlightBuilderAuthority !== undefined &&
    requestFlightBuilderAuthority !== defaultFlight.config.builderAuthority
  ) {
    // When the request overrides the builder, derive the collector for that
    // builder instead of reusing the configured default pair.
    return defaultFlight.resolveFeeCollectorTraderAddress(
      requestFlightBuilderAuthority
    );
  }

  return resolvePhoenixFlightFeeCollectorTraderAddress(
    defaultFlight.config,
    () => defaultFlight.resolveFeeCollectorTraderAddress()
  );
};

interface FlightOrderRoutingRequest {
  flightBuilderAuthority?: string | null;
  flightFeeCollectorTrader?: string | null;
}

const withDefaultFlightOrderRouting = async <
  T extends FlightOrderRoutingRequest,
>(
  request: T,
  defaultFlight: V1OrdersClientConfig["defaultFlight"]
): Promise<T> => {
  if (!defaultFlight) {
    return request;
  }

  const flightBuilderAuthority =
    request.flightBuilderAuthority ?? defaultFlight.config.builderAuthority;
  const requestFlightBuilderAuthority =
    request.flightBuilderAuthority ?? undefined;
  const flightFeeCollectorTrader =
    request.flightFeeCollectorTrader ??
    (await resolveDefaultFlightFeeCollectorTrader(
      defaultFlight,
      requestFlightBuilderAuthority as Authority | undefined
    ));

  return {
    ...request,
    flightBuilderAuthority,
    flightFeeCollectorTrader,
  };
};

export class V1OrdersClient {
  constructor(
    private readonly http: HttpTransport,
    private readonly config: V1OrdersClientConfig = {}
  ) {}

  async getTraderOrderHistory(
    authority: string,
    request?: OrderHistoryRequest
  ): Promise<OrderHistoryResponse> {
    return get(
      this.http,
      `/trader/${encodeURIComponent(authority)}/order-history`,
      OrderHistoryResponseSchema,
      { params: buildOrderHistoryQuery(request) }
    );
  }

  async getTraderOrderHistoryV2(
    traderPubkey: string,
    request?: OrderHistoryV2Request
  ): Promise<OrderHistoryV2Response> {
    return get(
      this.http,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/orders_v2`,
      OrderHistoryV2ResponseSchema,
      { params: buildOrderHistoryV2Query(request) }
    );
  }

  async cancelConditionalOrder(
    request: CancelConditionalOrderRequest
  ): Promise<InstructionsWithAccountsAndData[]> {
    return (
      await post(
        this.http,
        "/v1/ix/cancel-conditional-order",
        ApiInstructionResponseSchema.array(),
        CancelConditionalOrderRequestSchema.parse(request)
      )
    ).map(toInstruction);
  }

  async placeStopLossOrder(
    request: PlaceStopLossOrderRequest
  ): Promise<InstructionsWithAccountsAndData[]> {
    const routedRequest = await withDefaultFlightOrderRouting(
      request,
      this.config.defaultFlight
    );
    return (
      await post(
        this.http,
        "/v1/ix/place-stop-loss-order",
        ApiInstructionResponseSchema.array(),
        PlaceStopLossOrderRequestSchema.parse(routedRequest)
      )
    ).map(toInstruction);
  }

  async cancelStopLossOrder(
    request: CancelStopLossOrderRequest
  ): Promise<InstructionsWithAccountsAndData[]> {
    return (
      await post(
        this.http,
        "/v1/ix/cancel-stop-loss-order",
        ApiInstructionResponseSchema.array(),
        CancelStopLossOrderRequestSchema.parse(request)
      )
    ).map(toInstruction);
  }

  async placeAttachedConditionalOrder(
    request: PlaceAttachedConditionalOrderRequest
  ): Promise<InstructionsWithAccountsAndData[]> {
    const routedRequest = await withDefaultFlightOrderRouting(
      request,
      this.config.defaultFlight
    );
    return (
      await post(
        this.http,
        "/v1/ix/place-attached-conditional-order",
        ApiInstructionResponseSchema.array(),
        PlaceAttachedConditionalOrderRequestSchema.parse(routedRequest)
      )
    ).map(toInstruction);
  }

  async placePositionConditionalOrder(
    request: PlacePositionConditionalOrderRequest
  ): Promise<InstructionsWithAccountsAndData[]> {
    const routedRequest = await withDefaultFlightOrderRouting(
      request,
      this.config.defaultFlight
    );
    return (
      await post(
        this.http,
        "/v1/ix/place-position-conditional-order",
        ApiInstructionResponseSchema.array(),
        PlacePositionConditionalOrderRequestSchema.parse(routedRequest)
      )
    ).map(toInstruction);
  }

  async placeIsolatedLimitOrder(
    request: PlaceIsolatedLimitOrderRequest
  ): Promise<InstructionsWithAccountsAndData[]> {
    const routedRequest = await withDefaultFlightOrderRouting(
      request,
      this.config.defaultFlight
    );
    return (
      await post(
        this.http,
        "/v1/ix/place-isolated-limit-order",
        ApiInstructionResponseSchema.array(),
        PlaceIsolatedLimitOrderRequestSchema.parse(routedRequest)
      )
    ).map(toInstruction);
  }

  async placeIsolatedLimitOrderWithConditionals(
    request: PlaceIsolatedLimitOrderWithConditionalsRequest
  ): Promise<InstructionsWithAccountsAndData[]> {
    const routedRequest = await withDefaultFlightOrderRouting(
      request,
      this.config.defaultFlight
    );
    return (
      await post(
        this.http,
        "/v1/ix/place-isolated-limit-order-with-conditionals",
        ApiInstructionResponseSchema.array(),
        PlaceIsolatedLimitOrderWithConditionalsRequestSchema.parse(
          routedRequest
        )
      )
    ).map(toInstruction);
  }

  async placeIsolatedLimitOrderEnhanced(
    request: PlaceIsolatedLimitOrderRequest
  ): Promise<PlaceIsolatedOrderEnhancedInstructionsResponse> {
    const routedRequest = await withDefaultFlightOrderRouting(
      request,
      this.config.defaultFlight
    );
    const response = await post(
      this.http,
      "/v1/ix/place-isolated-limit-order-enhanced",
      PlaceIsolatedOrderEnhancedResponseSchema,
      PlaceIsolatedLimitOrderRequestSchema.parse(routedRequest)
    );

    return {
      instructions: response.instructions.map(toInstruction),
      estimatedLiquidationPriceUsd: response.estimatedLiquidationPriceUsd,
    };
  }

  async placeIsolatedMarketOrder(
    request: PlaceIsolatedMarketOrderRequest
  ): Promise<InstructionsWithAccountsAndData[]> {
    const routedRequest = await withDefaultFlightOrderRouting(
      request,
      this.config.defaultFlight
    );
    return (
      await post(
        this.http,
        "/v1/ix/place-isolated-market-order",
        ApiInstructionResponseSchema.array(),
        PlaceIsolatedMarketOrderRequestSchema.parse(routedRequest)
      )
    ).map(toInstruction);
  }

  async placeIsolatedMarketOrderEnhanced(
    request: PlaceIsolatedMarketOrderRequest
  ): Promise<PlaceIsolatedOrderEnhancedInstructionsResponse> {
    const routedRequest = await withDefaultFlightOrderRouting(
      request,
      this.config.defaultFlight
    );
    const response = await post(
      this.http,
      "/v1/ix/place-isolated-market-order-enhanced",
      PlaceIsolatedOrderEnhancedResponseSchema,
      PlaceIsolatedMarketOrderRequestSchema.parse(routedRequest)
    );

    return {
      instructions: response.instructions.map(toInstruction),
      estimatedLiquidationPriceUsd: response.estimatedLiquidationPriceUsd,
    };
  }
}
