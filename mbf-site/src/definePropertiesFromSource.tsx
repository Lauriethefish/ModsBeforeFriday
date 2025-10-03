/**
 * Define properties on a target object that forward to the source object's properties.
 */
export function definePropertiesFromSource<
  TSource extends object,
  TTarget extends object,
>(
  target: TTarget,
  source: TSource,
  keys?: Array<keyof TSource>,
  readonly: boolean = false
) {
  const propKeys = keys ?? (Object.keys(source) as Array<keyof TSource>);
  for (const key of propKeys) {
    Object.defineProperty(target, String(key), {
      get() {
        return (source as any)[key];
      },
      ...(readonly
        ? {}
        : {
            set(value: any) {
              (source as any)[key] = value;
            },
          }),
      enumerable: true,
      configurable: false,
    });
  }
}
