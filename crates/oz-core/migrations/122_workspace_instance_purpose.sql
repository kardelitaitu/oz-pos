-- 122_workspace_instance_purpose.sql
--
-- Topology business-logic builder: keep a controlled business purpose
-- separate from the technical workspace type, editable instance label, and
-- authorization/RBAC assignments. Existing instances receive the explicit
-- neutral purpose `general`; the topology compiler validates the supported
-- type/purpose matrix before applying a graph.

ALTER TABLE workspace_instances
    ADD COLUMN purpose_key TEXT NOT NULL DEFAULT 'general';
