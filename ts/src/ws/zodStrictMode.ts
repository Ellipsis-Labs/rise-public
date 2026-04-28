import * as z from "zod";

const getTypeName = (schema: z.ZodType): string | undefined =>
  (
    schema as {
      _def?: { typeName?: string; type?: string };
    }
  )._def?.typeName ?? (schema as { _def?: { type?: string } })._def?.type;

const isTypeName = (schema: z.ZodType, typeName: string): boolean =>
  getTypeName(schema) === typeName;

const isTypeNameOneOf = (schema: z.ZodType, names: string[]): boolean =>
  names.some((name) => isTypeName(schema, name));

const isPipeSchema = (schema: z.ZodType): boolean =>
  isTypeName(schema, "pipe") || isTypeName(schema, "ZodPipeline");

type AnyZod = z.ZodTypeAny;

export function applyStrictModeRecursive<Shape extends z.ZodRawShape>(
  schema: z.ZodObject<Shape>
): z.ZodObject<Shape>;

export function applyStrictModeRecursive<Element extends z.ZodType>(
  schema: z.ZodArray<Element>
): z.ZodArray<Element>;

export function applyStrictModeRecursive<Inner extends z.ZodType>(
  schema: z.ZodOptional<Inner>
): z.ZodOptional<Inner>;

export function applyStrictModeRecursive<Inner extends z.ZodType>(
  schema: z.ZodNullable<Inner>
): z.ZodNullable<Inner>;

export function applyStrictModeRecursive<
  Options extends [z.ZodType, z.ZodType, ...z.ZodType[]],
>(schema: z.ZodUnion<Options>): z.ZodUnion<Options>;

export function applyStrictModeRecursive<T extends z.ZodType>(schema: T): T;

export function applyStrictModeRecursive(schema: z.ZodType): z.ZodType {
  if (schema instanceof z.ZodObject) {
    const shape = schema.shape as z.ZodRawShape;
    const newShape: Record<string, z.ZodType> = {};

    for (const [key, fieldSchema] of Object.entries(shape) as Array<
      [string, z.ZodType]
    >) {
      newShape[key] = applyStrictModeRecursive(fieldSchema);
    }

    const newSchema = z.object(newShape);
    return newSchema.strict();
  }

  if (isTypeNameOneOf(schema, ["ZodEffects", "effects"])) {
    const def = schema._def as unknown as { schema: z.ZodType };
    const innerSchema = def.schema;
    const strictInner = applyStrictModeRecursive(innerSchema);
    const ctor = schema.constructor as unknown as new (
      def: unknown
    ) => z.ZodTypeAny;
    return new ctor({
      ...def,
      schema: strictInner,
    });
  }

  if (isPipeSchema(schema)) {
    const def = schema._def as unknown as { in: z.ZodType; out: z.ZodType };
    const strictIn = applyStrictModeRecursive(def.in);
    const strictOut = applyStrictModeRecursive(def.out);
    const ctor = schema.constructor as unknown as new (
      def: unknown
    ) => z.ZodTypeAny;
    return new ctor({
      ...def,
      in: strictIn,
      out: strictOut,
    });
  }

  if (schema instanceof z.ZodArray) {
    const newElement = applyStrictModeRecursive(schema.element as z.ZodType);
    return z.array(newElement);
  }

  if (schema instanceof z.ZodOptional) {
    return applyStrictModeRecursive(schema.unwrap() as z.ZodType).optional();
  }

  if (schema instanceof z.ZodNullable) {
    return applyStrictModeRecursive(schema.unwrap() as z.ZodType).nullable();
  }

  if (schema instanceof z.ZodUnion) {
    const newOptions = (schema.options as z.ZodType[]).map((opt) =>
      applyStrictModeRecursive(opt)
    );
    return z.union(newOptions as [AnyZod, AnyZod, ...AnyZod[]]);
  }

  if (isTypeNameOneOf(schema, ["ZodIntersection", "intersection"])) {
    const def = schema._def as unknown as { left: z.ZodType; right: z.ZodType };
    const left = applyStrictModeRecursive(def.left);
    const right = applyStrictModeRecursive(def.right);
    return z.intersection(left, right);
  }

  if (isTypeNameOneOf(schema, ["ZodTuple", "tuple"])) {
    const def = schema._def as unknown as { items: z.ZodType[] };
    const newItems = def.items.map((item) => applyStrictModeRecursive(item));
    return z.tuple(newItems as [AnyZod, ...AnyZod[]]);
  }

  if (isTypeNameOneOf(schema, ["ZodRecord", "record"])) {
    const def = schema._def as unknown as {
      valueType: AnyZod;
    };
    const valueType = applyStrictModeRecursive(def.valueType);
    return z.record(z.string(), valueType);
  }

  if (isTypeNameOneOf(schema, ["ZodLazy", "lazy"])) {
    const def = schema._def as unknown as { getter: () => z.ZodType };
    const getter = def.getter;
    return z.lazy(() => applyStrictModeRecursive(getter()));
  }

  return schema;
}
