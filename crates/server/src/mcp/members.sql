SELECT jsonb_build_object(
    'account_id',a.id,'username',a.username,'enabled',a.enabled,
    'brain_role',recollect_mcp_account_brain_role($1,a.id),
    'rights',jsonb_build_object(
        'use_profile',recollect_mcp_account_can($1,a.id,'use'),
        'manage',recollect_mcp_account_can($1,a.id,'manage'),
        'share',recollect_mcp_account_can($1,a.id,'share')),
    'direct_rights',jsonb_build_object(
        'use_profile',coalesce(d.can_use,false),
        'manage',coalesce(d.can_manage,false),
        'share',coalesce(d.can_share,false)),
    'groups',coalesce((SELECT jsonb_agg(g.group_name ORDER BY g.group_name)
        FROM mcp_profile_grants g
        WHERE g.profile_id=$1 AND g.account_id IS NULL AND a.auth_kind='oidc'
          AND a.membership_until>now() AND g.issuer=a.oidc_issuer
          AND g.group_name=ANY(a.oidc_groups)),'[]'::jsonb),
    'membership_until',a.membership_until)
FROM accounts a
LEFT JOIN mcp_profile_grants d ON d.profile_id=$1 AND d.account_id=a.id
WHERE a.id=$2 OR d.id IS NOT NULL
    OR recollect_mcp_account_can($1,a.id,'manage')
    OR recollect_mcp_account_can($1,a.id,'share')
    OR EXISTS(SELECT 1 FROM mcp_profile_grants g
              WHERE g.profile_id=$1 AND g.account_id IS NULL AND a.auth_kind='oidc'
                AND g.issuer=a.oidc_issuer AND g.group_name=ANY(a.oidc_groups))
ORDER BY a.username,a.id LIMIT 1001
