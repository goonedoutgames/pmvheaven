import { LibraryList } from "@/components/LibraryList";

export default function FavoritesPage() {
  return (
    <LibraryList
      endpoint="/api/favorites"
      title="Favorites"
      emptyLabel="You haven't favorited any videos yet."
    />
  );
}
