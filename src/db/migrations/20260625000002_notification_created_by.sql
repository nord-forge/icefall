-- Record who created each notification channel, for the dashboard subtext
-- ("Added {relative time} by {email}"). Nullable because existing channels
-- predate this column — those render as "by unknown". We store the creator's
-- email as a denormalized snapshot so the label survives user deletion and
-- needs no join at list time.

ALTER TABLE notifications ADD COLUMN created_by TEXT;
