import { LibraryList } from "@/components/LibraryList";

export default function WatchLaterPage() {
  return (
    <LibraryList
      endpoint="/api/watch-later"
      title="Watch Later"
      emptyLabel="Your watch later queue is empty."
    />
  );
}
