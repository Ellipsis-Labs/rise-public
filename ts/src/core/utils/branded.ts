declare const __brand: unique symbol;
type Brand<B extends string> = { readonly [__brand]: B };
export type Branded<T, B extends string> = T & Brand<B>;
