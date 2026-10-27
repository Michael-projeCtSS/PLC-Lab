import { useCallback, useEffect, useState } from "react";

/** Load data when its key changes; `loading` is true until the current key resolves. */
export function useLoad<T>(
  load: () => Promise<T>,
  deps: readonly (string | number | null)[],
) {
  const [nonce, setNonce] = useState(0);
  const key = `${JSON.stringify(deps)}#${nonce}`;
  const [state, setState] = useState<{
    key: string | null;
    data: T | null;
    error: unknown;
  }>({ key: null, data: null, error: null });
  useEffect(() => {
    let active = true;
    load().then(
      (data) => active && setState({ key, data, error: null }),
      (error: unknown) => active && setState({ key, data: null, error }),
    );
    return () => {
      active = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  const reload = useCallback(() => setNonce((n) => n + 1), []);
  return {
    data: state.data,
    error: state.error,
    loading: state.key !== key,
    reload,
  };
}
