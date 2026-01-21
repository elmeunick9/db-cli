-- @requires /auth/default.sql
INSERT INTO "story" 
    ("id", "created_at", "updated_at", "author_id", "title", "subtitle", "summary", "views", "image", "media", "approved") 
VALUES
    (
        '432c4909-c773-48b9-841d-c29b0ec7ab41', 
        '2023-09-26 15:37:12.229658', 
        '2023-10-03 16:19:48.258+00',
        '604b0347-83db-4bda-a6fa-81f610c46a8d',
        'The Demo Game',
        'A demo story for a demo site where nothing much happens, super long long line I tell you, and there is even more.',
        'Empty for now.',
        0,
        'story-feb5a4a5-4fa6-48b1-97ad-cbca00f0daa0.jpg',
        'b6ffb2ce-f248-463f-a082-80e9662d8776',
        true
    );

INSERT INTO "comment" 
    ("story", "author", "message") 
VALUES
    (
        '432c4909-c773-48b9-841d-c29b0ec7ab41',
        '604b0347-83db-4bda-a6fa-81f610c46a8d',
        'Such a cool story!'
    );