-- @requires ./schema.sql
CREATE FUNCTION "comment_vote" (comment_id uuid, author_id uuid /* of the vote */, direction smallint)
RETURNS VOID AS $$
DECLARE
    new_vote vote_direction;
    old_vote vote_direction;
    rank_change int4;
BEGIN
    -- Validate direction parameter
    IF direction NOT IN (-1, 0, 1) THEN
        RAISE EXCEPTION 'Invalid direction value: %. Must be -1, 0, or 1.', direction;
    END IF;

    -- Get the old vote if it exists
    SELECT "vote" INTO old_vote FROM "comment_action" 
    WHERE "id" = comment_id AND "author" = author_id;

    -- Handle direction = 0 (remove vote)
    IF direction = 0 THEN
        IF old_vote IS NOT NULL THEN
            -- Calculate rank change (inverse of the vote)
            rank_change := CASE WHEN old_vote = 'up' THEN -1 ELSE 1 END;
            
            -- Delete the vote
            DELETE FROM "comment_action" 
            WHERE "id" = comment_id AND "author" = author_id;
            
            -- Update comment rank
            UPDATE "comment" SET "rank" = "rank" + rank_change WHERE "id" = comment_id;
        END IF;
    ELSE
        -- Convert direction to vote_direction
        new_vote := CASE WHEN direction = 1 THEN 'up'::vote_direction ELSE 'down'::vote_direction END;
        
        -- Calculate rank change
        IF old_vote IS NULL THEN
            -- New vote
            rank_change := CASE WHEN new_vote = 'up' THEN 1 ELSE -1 END;
        ELSE
            -- Changing existing vote
            rank_change := CASE 
                WHEN old_vote = 'up' AND new_vote = 'down' THEN -2
                WHEN old_vote = 'down' AND new_vote = 'up' THEN 2
                ELSE 0
            END;
        END IF;
        
        -- Upsert the vote
        INSERT INTO "comment_action" ("id", "author", "vote")
        VALUES (comment_id, author_id, new_vote)
        ON CONFLICT ("id", "author") DO UPDATE SET "vote" = new_vote;
        
        -- Update comment rank
        IF rank_change != 0 THEN
            UPDATE "comment" SET "rank" = "rank" + rank_change WHERE "id" = comment_id;
        END IF;
    END IF;
END;
$$ LANGUAGE plpgsql;