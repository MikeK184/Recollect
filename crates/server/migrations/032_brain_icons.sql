-- A Brain icon is small presentation data owned by the canonical Brain row.
-- Existing RLS, deletion and restored-backup journal replay apply unchanged.
ALTER TABLE brains ADD COLUMN icon_png bytea;
ALTER TABLE brains ADD COLUMN icon_revision uuid;
ALTER TABLE brains ADD CONSTRAINT brain_icon_bounded CHECK (
    (icon_png IS NULL AND icon_revision IS NULL) OR
    (icon_png IS NOT NULL AND icon_revision IS NOT NULL
     AND octet_length(icon_png) BETWEEN 1 AND 524288)
);
