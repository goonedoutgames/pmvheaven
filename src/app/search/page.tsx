import { InfiniteFeed } from "@/components/InfiniteFeed";

export const dynamic = "force-dynamic";

export default async function SearchPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const sp = await searchParams;
  const q = (Array.isArray(sp.q) ? sp.q[0] : sp.q)?.trim() ?? "";

  return (
    <div className="flex flex-col gap-5">
      <h1 className="text-2xl font-bold tracking-tight">
        {q ? (
          <>
            Results for <span className="text-accent">“{q}”</span>
          </>
        ) : (
          "Search"
        )}
      </h1>
      {q ? (
        <InfiniteFeed
          endpoint="/api/search"
          params={{ q }}
          emptyLabel={`No results for “${q}”.`}
        />
      ) : (
        <p className="py-16 text-center text-muted">
          Type a query in the search bar to find videos, tags, and creators.
        </p>
      )}
    </div>
  );
}
