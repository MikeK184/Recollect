-- Operator approval is also available to the authenticated installation owner.
-- Runtime callers retain metadata reads; no UPDATE or DELETE grant is added.
ALTER TABLE mcp_definitions ENABLE ROW LEVEL SECURITY;
CREATE POLICY definition_read ON mcp_definitions FOR SELECT USING (true);
CREATE POLICY definition_owner_create ON mcp_definitions FOR INSERT WITH CHECK (
    approved_by = recollect_actor() AND recollect_device() IS NULL AND
    EXISTS (SELECT 1 FROM accounts WHERE id=recollect_actor() AND enabled AND installation_owner)
);
GRANT INSERT ON mcp_definitions TO recollect_app;
