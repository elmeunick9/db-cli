ALTER TABLE "story" DROP CONSTRAINT IF EXISTS "story_author_fkey";

ALTER TABLE "story" RENAME COLUMN "author" TO "author_id";

ALTER TABLE "story" ADD COLUMN "author" boolean NOT NULL DEFAULT false;

ALTER TABLE "story" ADD CONSTRAINT "story_author_id_fkey" FOREIGN KEY ("author_id") REFERENCES "auth"."user" ("id");

ALTER TABLE "story_genre" ALTER COLUMN "description" SET DEFAULT '';