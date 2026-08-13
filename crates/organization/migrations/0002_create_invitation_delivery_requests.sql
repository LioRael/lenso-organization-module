create table if not exists organization.invitation_delivery_requests (
    id text primary key,
    organization_id text not null references organization.organizations(id) on delete cascade,
    idempotency_key text not null,
    request_digest text not null,
    invitation_id text references organization.invitations(id) on delete restrict,
    notification_intent_id text,
    notification_delivery_id text,
    initial_delivery_status text,
    created_at timestamptz not null,
    completed_at timestamptz,
    constraint invitation_delivery_requests_idempotency_key_not_empty
        check (length(idempotency_key) between 1 and 300),
    constraint invitation_delivery_requests_digest_not_empty
        check (length(request_digest) > 0),
    constraint invitation_delivery_requests_completion_consistent check (
        (completed_at is null
            and invitation_id is null
            and notification_intent_id is null
            and notification_delivery_id is null
            and initial_delivery_status is null)
        or
        (completed_at is not null
            and invitation_id is not null
            and notification_intent_id is not null
            and notification_delivery_id is not null
            and initial_delivery_status is not null)
    )
);

create unique index if not exists invitation_delivery_requests_organization_key
    on organization.invitation_delivery_requests (organization_id, idempotency_key);

create index if not exists invitation_delivery_requests_invitation_id_idx
    on organization.invitation_delivery_requests (invitation_id)
    where invitation_id is not null;
