-- @requires ./comment_vote.function.sql
CREATE FUNCTION "comment_vote_test" ()
RETURNS text AS $$
DECLARE
    test_comment_id uuid;
    test_story_id uuid;
    test_author1 uuid;
    test_author2 uuid;
    test_user_id uuid;
    initial_rank int4;
    current_rank int4;
    vote_exists boolean;
    vote_value vote_direction;
    msg text;
BEGIN
    -- Create test user
    INSERT INTO "auth"."user" ("email") VALUES ('testuser@example.com') RETURNING "id" INTO test_user_id;

    -- Create test story
    INSERT INTO "story" ("author", "title") 
    VALUES (test_user_id, 'Test Story') 
    RETURNING "id" INTO test_story_id;

    -- Create test comment
    INSERT INTO "comment" ("story", "author", "message") 
    VALUES (test_story_id, test_user_id, 'Test Comment') 
    RETURNING "id" INTO test_comment_id;

    -- Create test vote authors
    INSERT INTO "auth"."user" ("email") VALUES ('author1@example.com') RETURNING "id" INTO test_author1;
    INSERT INTO "auth"."user" ("email") VALUES ('author2@example.com') RETURNING "id" INTO test_author2;

    -- Get initial rank
    SELECT "rank" INTO initial_rank FROM "comment" WHERE "id" = test_comment_id;
    IF initial_rank IS NULL OR initial_rank != 0 THEN
        RETURN format('[432c4909] initial_rank=%s expected=0', initial_rank);
    END IF;

    -- TEST: Add upvote
    PERFORM "comment_vote"(test_comment_id, test_author1, 1::smallint);
    SELECT "rank" INTO current_rank FROM "comment" WHERE "id" = test_comment_id;
    IF current_rank != 1 THEN
        RETURN format('[b1e2c3d4] rank=%s expected=1', current_rank);
    END IF;
    SELECT "vote" INTO vote_value FROM "comment_action" WHERE "id" = test_comment_id AND "author" = test_author1;
    IF vote_value != 'up' THEN
        RETURN format('[c2d3e4f5] vote=%s expected=up', vote_value);
    END IF;

    -- TEST: Add another upvote (rank should become 2)
    PERFORM "comment_vote"(test_comment_id, test_author2, 1::smallint);
    SELECT "rank" INTO current_rank FROM "comment" WHERE "id" = test_comment_id;
    IF current_rank != 2 THEN
        RETURN format('[b2e2c3d5] rank=%s expected=2', current_rank);
    END IF;

    -- TEST: Change upvote to downvote
    PERFORM "comment_vote"(test_comment_id, test_author1, -1::smallint);
    SELECT "rank" INTO current_rank FROM "comment" WHERE "id" = test_comment_id;
    IF current_rank != 0 THEN
        RETURN format('[d3e4f5a6] rank=%s expected=0', current_rank);
    END IF;
    

    -- TEST: Change downvote back to upvote
    PERFORM "comment_vote"(test_comment_id, test_author1, 1::smallint);
    SELECT "rank" INTO current_rank FROM "comment" WHERE "id" = test_comment_id;
    IF current_rank != 2 THEN
        RETURN format('[f5a6b7c8] rank=%s expected=2', current_rank);
    END IF;
    SELECT "vote" INTO vote_value FROM "comment_action" WHERE "id" = test_comment_id AND "author" = test_author1;
    IF vote_value != 'up' THEN
        RETURN format('[a6b7c8d9] vote=%s expected=up', vote_value);
    END IF;

    -- TEST: Remove vote
    PERFORM "comment_vote"(test_comment_id, test_author1, 0::smallint);
    SELECT "rank" INTO current_rank FROM "comment" WHERE "id" = test_comment_id;
    IF current_rank != 1 THEN
        RETURN format('[b7c8d9e0] rank=%s expected=1', current_rank);
    END IF;
    SELECT EXISTS(SELECT 1 FROM "comment_action" WHERE "id" = test_comment_id AND "author" = test_author1) INTO vote_exists;
    IF vote_exists THEN
        RETURN format('[c8d9e0f1] vote_exists=%s', vote_exists);
    END IF;

    -- TEST: Remove vote that doesn't exist (should be idempotent)
    PERFORM "comment_vote"(test_comment_id, test_author1, 0::smallint);
    SELECT "rank" INTO current_rank FROM "comment" WHERE "id" = test_comment_id;
    IF current_rank != 1 THEN
        RETURN format('[d9e0f1a2] rank=%s expected=1', current_rank);
    END IF;

    -- TEST: Test downvote
    PERFORM "comment_vote"(test_comment_id, test_author2, 0::smallint);
    PERFORM "comment_vote"(test_comment_id, test_author1, -1::smallint);
    SELECT "rank" INTO current_rank FROM "comment" WHERE "id" = test_comment_id;
    IF current_rank != -1 THEN
        RETURN format('[e0f1a2b3] rank=%s expected=-1', current_rank);
    END IF;
    SELECT "vote" INTO vote_value FROM "comment_action" WHERE "id" = test_comment_id AND "author" = test_author1;
    IF vote_value != 'down' THEN
        RETURN format('[f1a2b3c4] vote=%s expected=down', vote_value);
    END IF;

    -- TEST: Remove downvote
    PERFORM "comment_vote"(test_comment_id, test_author1, 0::smallint);
    SELECT "rank" INTO current_rank FROM "comment" WHERE "id" = test_comment_id;
    IF current_rank != 0 THEN
        RETURN format('[a2b3c4d5] rank=%s expected=0', current_rank);
    END IF;

    -- Cleanup (only on success)
    -- DELETE FROM "comment_action" WHERE "id" = test_comment_id;
    -- DELETE FROM "comment" WHERE "id" = test_comment_id;
    -- DELETE FROM "story" WHERE "id" = test_story_id;
    -- DELETE FROM "auth"."user" WHERE "id" IN (test_user_id, test_author1, test_author2);

    RETURN 'OK';
END;
$$ LANGUAGE plpgsql;
