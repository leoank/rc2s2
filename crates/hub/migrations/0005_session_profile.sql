-- Phase 3: the launch profile a spawned session runs under (ADR-0038); empty for the host.

ALTER TABLE sessions ADD COLUMN profile TEXT NOT NULL DEFAULT '';
