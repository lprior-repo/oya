export type Effect<A> = () => Promise<A>;

export const succeed = <A>(value: A): Effect<A> => async () => value;

export const fromPromise = <A>(thunk: () => Promise<A>): Effect<A> => thunk;

export const flatMap = <A, B>(
  effect: Effect<A>,
  f: (value: A) => Effect<B>,
): Effect<B> => {
  return async () => {
    const value = await effect();
    return f(value)();
  };
};

export const map = <A, B>(effect: Effect<A>, f: (value: A) => B): Effect<B> => {
  return async () => f(await effect());
};

export const zipRight = <A, B>(left: Effect<A>, right: Effect<B>): Effect<B> => {
  return async () => {
    await left();
    return right();
  };
};

export const tap = <A>(effect: Effect<A>, f: (value: A) => Promise<void>): Effect<A> => {
  return async () => {
    const value = await effect();
    await f(value);
    return value;
  };
};

export async function runPromise<A>(effect: Effect<A>): Promise<A> {
  return effect();
}

export const all = <A>(effects: ReadonlyArray<Effect<A>>): Effect<ReadonlyArray<A>> => {
  return async () => Promise.all(effects.map((effect) => effect()));
};
