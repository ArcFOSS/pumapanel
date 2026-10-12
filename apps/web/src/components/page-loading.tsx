interface PageLoadingProps {
  message?: string;
}

/** Centered spinner shown while a page resolves its session state. */
export const PageLoading = ({ message = "Loading…" }: PageLoadingProps) => (
  <output className="flex flex-1 items-center justify-center p-10">
    <span className="flex flex-col items-center gap-3">
      <span className="border-muted-foreground/30 border-t-foreground size-6 animate-spin rounded-full border-2" />
      <span className="text-muted-foreground text-sm">{message}</span>
    </span>
  </output>
);
