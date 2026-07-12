import Link from "next/link";
import { InfiniteFeed } from "@/components/InfiniteFeed";

export const dynamic = "force-dynamic";

const SORT_TABS = [
  { label: "Newest", sort: "-uploadDate" },
  { label: "Most viewed", sort: "-views" },
  { label: "Top rated", sort: "-bayesianRating" },
  { label: "Most liked", sort: "-likes" },
];

export default async function BrowsePage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const sp = await searchParams;
  const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v);
  const sort = one(sp.sort) ?? "-uploadDate";
  const tags = one(sp.tags);
  const creator = one(sp.creator);

  const params: Record<string, string> = { sort };
  if (tags) params.tags = tags;
  if (creator) params.creator = creator;

  const heading = tags
    ? `#${tags}`
    : creator
      ? creator
      : "Browse";

  const buildHref = (s: string) => {
    const qs = new URLSearchParams(params);
    qs.set("sort", s);
    return `/browse?${qs}`;
  };

  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-col gap-3">
        <h1 className="text-2xl font-bold capitalize tracking-tight">{heading}</h1>
        <div className="flex flex-wrap gap-2">
          {SORT_TABS.map((tab) => (
            <Link
              key={tab.sort}
              href={buildHref(tab.sort)}
              className={`rounded-full border px-3.5 py-1.5 text-sm font-medium transition ${
                sort === tab.sort
                  ? "border-accent bg-accent/15 text-foreground"
                  : "border-border bg-surface text-muted hover:text-foreground"
              }`}
            >
              {tab.label}
            </Link>
          ))}
        </div>
      </div>

      <InfiniteFeed endpoint="/api/feed" params={params} />
    </div>
  );
}
