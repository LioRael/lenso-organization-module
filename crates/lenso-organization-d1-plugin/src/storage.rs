//! Every mutation, including owner creation, carries admission in its one primary batch.
use lenso_migration_d1::{Error, Statement, Transport};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Mutation {
    Create,
    Add,
    Remove,
}
impl Mutation {
    fn family(self) -> &'static str {
        match self {
            Self::Create => "creation",
            Self::Add | Self::Remove => "membership",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Create => "create_organization",
            Self::Add => "add_member",
            Self::Remove => "remove_member",
        }
    }
}

/// Only the Plugin can construct business batches; the Host supplies storage I/O.
pub(crate) async fn mutate<R: DeserializeOwned, E: DeserializeOwned>(
    db: &impl Transport,
    operation: Mutation,
    caller: Option<&str>,
    allowed: &[String],
    request: &impl Serialize,
    invalid: Option<&str>,
) -> Result<Result<R, E>, Error> {
    let mut request = serde_json::to_value(request).map_err(|_| Error::Transport)?;
    if let Some(name) = request.get_mut("name") {
        *name = json!(name.as_str().ok_or(Error::Transport)?.trim());
    }
    let mut intent = request.clone();
    let object = intent.as_object_mut().ok_or(Error::Transport)?;
    object.remove("idempotency_key");
    object.insert("operation".into(), json!(operation.name()));
    let mut batch = vec![
        sql("DELETE FROM organization_operation"),
        Statement::new(
            "INSERT INTO organization_operation(singleton,caller,family,request,intent,error) VALUES(1,?1,?2,?3,?4,CASE WHEN ?1 IS NULL OR NOT EXISTS(SELECT 1 FROM json_each(?5) WHERE type='text' AND value COLLATE BINARY = ?1 COLLATE BINARY) THEN 'forbidden' ELSE ?6 END)",
            vec![
                json!(caller),
                json!(operation.family()),
                json!(request.to_string()),
                json!(intent.to_string()),
                json!(serde_json::to_string(allowed).map_err(|_| Error::Transport)?),
                json!(invalid),
            ],
        ),
        sql(
            "UPDATE organization_operation AS w SET error='idempotency_conflict' WHERE error IS NULL AND EXISTS(SELECT 1 FROM organization_receipts r WHERE r.caller=w.caller AND r.family=w.family AND r.idempotency_key=json_extract(w.request,'$.idempotency_key') AND r.intent<>w.intent)",
        ),
        Statement::new(
            "UPDATE organization_operation AS w SET response=(SELECT json_set(r.response,?1,json('false')) FROM organization_receipts r WHERE r.caller=w.caller AND r.family=w.family AND r.idempotency_key=json_extract(w.request,'$.idempotency_key')) WHERE error IS NULL",
            vec![json!(if matches!(operation, Mutation::Remove) {
                "$.removed"
            } else {
                "$.created"
            })],
        ),
    ];
    batch.extend(
        match operation {
            Mutation::Create => CREATE,
            Mutation::Add => ADD,
            Mutation::Remove => REMOVE,
        }
        .iter()
        .map(|s| sql(s)),
    );
    batch.push(sql("INSERT INTO organization_receipts(caller,family,idempotency_key,intent,response) SELECT caller,family,json_extract(request,'$.idempotency_key'),intent,response FROM organization_operation w WHERE error IS NULL AND NOT EXISTS(SELECT 1 FROM organization_receipts r WHERE r.caller=w.caller AND r.family=w.family AND r.idempotency_key=json_extract(w.request,'$.idempotency_key'))"));
    batch.push(sql("SELECT error,response FROM organization_operation"));
    batch.push(sql("DELETE FROM organization_operation"));
    let count = batch.len();
    let mut results = db.batch(batch).await?;
    if results.len() != count {
        return Err(Error::Transport);
    }
    decode(results.remove(count - 2))
}

fn sql(value: &str) -> Statement {
    Statement::new(value, vec![])
}

const CREATE: &[&str] = &[
    "UPDATE organization_operation SET error='slug_conflict' WHERE error IS NULL AND response IS NULL AND EXISTS(SELECT 1 FROM organizations WHERE slug=json_extract(request,'$.slug') AND active=1)",
    "UPDATE organization_operation SET organization_id='org_'||lower(hex(randomblob(18))),membership_id='member_'||lower(hex(randomblob(18))),fresh=1 WHERE error IS NULL AND response IS NULL",
    "INSERT INTO organizations(organization_id,name,slug) SELECT organization_id,json_extract(request,'$.name'),json_extract(request,'$.slug') FROM organization_operation WHERE fresh=1 AND error IS NULL",
    "INSERT INTO organization_memberships(membership_id,organization_id,subject,is_owner) SELECT membership_id,organization_id,json_extract(request,'$.owner_subject'),1 FROM organization_operation WHERE fresh=1 AND error IS NULL",
    "UPDATE organization_operation SET response=json_object('organization_id',organization_id,'owner_membership_id',membership_id,'created',json('true')) WHERE fresh=1 AND error IS NULL",
];
const ACTIVE_ORGANIZATION: &str = "UPDATE organization_operation SET error='organization_not_found' WHERE error IS NULL AND response IS NULL AND NOT EXISTS(SELECT 1 FROM organizations WHERE organization_id=json_extract(request,'$.organization_id') AND active=1)";
const MEMBER: &str = "UPDATE organization_operation SET membership_id=(SELECT membership_id FROM organization_memberships WHERE organization_id=json_extract(request,'$.organization_id') AND subject=json_extract(request,'$.subject') AND active=1),revision=(SELECT revision FROM organization_memberships WHERE organization_id=json_extract(request,'$.organization_id') AND subject=json_extract(request,'$.subject') AND active=1) WHERE error IS NULL AND response IS NULL";
const ADD: &[&str] = &[
    ACTIVE_ORGANIZATION,
    MEMBER,
    "UPDATE organization_operation SET membership_id='member_'||lower(hex(randomblob(18))),revision=1,fresh=1 WHERE error IS NULL AND response IS NULL AND membership_id IS NULL",
    "INSERT INTO organization_memberships(membership_id,organization_id,subject,is_owner) SELECT membership_id,json_extract(request,'$.organization_id'),json_extract(request,'$.subject'),0 FROM organization_operation WHERE fresh=1 AND error IS NULL",
    "UPDATE organization_operation SET response=json_object('membership_id',membership_id,'revision',CAST(revision AS TEXT),'created',json(CASE WHEN fresh=1 THEN 'true' ELSE 'false' END)) WHERE error IS NULL AND response IS NULL",
];
const REMOVE: &[&str] = &[
    ACTIVE_ORGANIZATION,
    MEMBER,
    "UPDATE organization_operation SET error='membership_not_found' WHERE error IS NULL AND response IS NULL AND membership_id IS NULL",
    "UPDATE organization_operation AS w SET error='owner_protected' WHERE error IS NULL AND response IS NULL AND EXISTS(SELECT 1 FROM organization_memberships m WHERE m.membership_id=w.membership_id AND m.is_owner=1)",
    // STRICT integer storage rejects overflow before any membership write.
    "UPDATE organization_operation SET revision=revision+1,fresh=1 WHERE error IS NULL AND response IS NULL",
    "UPDATE organization_memberships AS m SET active=0,revision=(SELECT revision FROM organization_operation) WHERE EXISTS(SELECT 1 FROM organization_operation w WHERE w.error IS NULL AND w.fresh=1 AND w.membership_id=m.membership_id)",
    "UPDATE organization_operation SET response=json_object('membership_id',membership_id,'revision',CAST(revision AS TEXT),'removed',json('true')) WHERE fresh=1 AND error IS NULL",
];

pub(crate) async fn read<R: DeserializeOwned, E: DeserializeOwned>(
    db: &impl Transport,
    query: &str,
    request: &impl Serialize,
) -> Result<Result<R, E>, Error> {
    let mut rows = db
        .batch(vec![Statement::new(
            query,
            vec![json!(
                serde_json::to_string(request).map_err(|_| Error::Transport)?
            )],
        )])
        .await?;
    if rows.len() != 1 {
        return Err(Error::Transport);
    }
    decode(rows.remove(0))
}
fn decode<R: DeserializeOwned, E: DeserializeOwned>(
    mut rows: Vec<Value>,
) -> Result<Result<R, E>, Error> {
    if rows.len() != 1 {
        return Err(Error::Transport);
    }
    let row = rows.remove(0);
    let error = row.get("error").ok_or(Error::Transport)?;
    if error.is_null() {
        Ok(Ok(serde_json::from_str(
            row["response"].as_str().ok_or(Error::Transport)?,
        )
        .map_err(|_| Error::Transport)?))
    } else {
        Ok(Err(
            serde_json::from_value(error.clone()).map_err(|_| Error::Transport)?
        ))
    }
}

pub(crate) const GET: &str = "WITH r AS (SELECT json_extract(?1,'$.organization_id') AS id) SELECT CASE WHEN o.organization_id IS NULL THEN 'organization_not_found' END AS error,json_object('organization_id',o.organization_id,'name',o.name,'slug',o.slug,'active',json(CASE WHEN o.active=1 THEN 'true' ELSE 'false' END),'revision',CAST(o.revision AS TEXT)) AS response FROM r LEFT JOIN organizations o ON o.organization_id=r.id";
pub(crate) const CHECK: &str = "WITH r AS (SELECT json_extract(?1,'$.organization_id') AS id,json_extract(?1,'$.subject') AS subject), m AS (SELECT active,is_owner FROM organization_memberships,r WHERE organization_id=r.id AND organization_memberships.subject=r.subject ORDER BY sequence DESC LIMIT 1) SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM organizations,r WHERE organization_id=r.id AND active=1) THEN 'organization_not_found' END AS error,json_object('active',json(CASE WHEN (SELECT active FROM m)=1 THEN 'true' ELSE 'false' END),'owner',json(CASE WHEN (SELECT active AND is_owner FROM m)=1 THEN 'true' ELSE 'false' END)) AS response";

pub(crate) const LIST_ORGANIZATIONS: &str = "WITH page AS (SELECT * FROM organizations WHERE (json_extract(?1,'$.status')='all' OR active=(json_extract(?1,'$.status')='active')) AND (json_extract(?1,'$.slug') IS NULL OR slug=json_extract(?1,'$.slug')) AND (json_extract(?1,'$.cursor') IS NULL OR organization_id COLLATE BINARY > json_extract(?1,'$.cursor')) ORDER BY organization_id COLLATE BINARY LIMIT json_extract(?1,'$.limit')+1), visible AS (SELECT * FROM page ORDER BY organization_id COLLATE BINARY LIMIT json_extract(?1,'$.limit')) SELECT NULL AS error,json_object('organizations',json((SELECT json_group_array(json_object('organization_id',organization_id,'name',name,'slug',slug,'active',json(CASE WHEN active=1 THEN 'true' ELSE 'false' END),'revision',CAST(revision AS TEXT))) FROM visible)),'next_cursor',CASE WHEN (SELECT count(*) FROM page)>json_extract(?1,'$.limit') THEN (SELECT organization_id FROM visible ORDER BY organization_id COLLATE BINARY DESC LIMIT 1) END) AS response";
pub(crate) const LIST_MEMBERS: &str = "WITH page AS (SELECT * FROM organization_memberships WHERE organization_id=json_extract(?1,'$.organization_id') AND (json_extract(?1,'$.status')='all' OR active=(json_extract(?1,'$.status')='active')) AND (json_extract(?1,'$.subject') IS NULL OR subject=json_extract(?1,'$.subject')) AND (json_extract(?1,'$.cursor') IS NULL OR membership_id COLLATE BINARY > json_extract(?1,'$.cursor')) ORDER BY membership_id COLLATE BINARY LIMIT json_extract(?1,'$.limit')+1), visible AS (SELECT * FROM page ORDER BY membership_id COLLATE BINARY LIMIT json_extract(?1,'$.limit')) SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM organizations WHERE organization_id=json_extract(?1,'$.organization_id')) THEN 'organization_not_found' END AS error,json_object('organization_id',json_extract(?1,'$.organization_id'),'members',json((SELECT json_group_array(json_object('membership_id',membership_id,'subject',subject,'active',json(CASE WHEN active=1 THEN 'true' ELSE 'false' END),'is_owner',json(CASE WHEN is_owner=1 THEN 'true' ELSE 'false' END),'revision',CAST(revision AS TEXT))) FROM visible)),'next_cursor',CASE WHEN (SELECT count(*) FROM page)>json_extract(?1,'$.limit') THEN (SELECT membership_id FROM visible ORDER BY membership_id COLLATE BINARY DESC LIMIT 1) END) AS response";
pub(crate) const LIST_FOR_SUBJECT: &str = "WITH page AS (SELECT o.* FROM organizations o JOIN organization_memberships m ON m.organization_id=o.organization_id WHERE o.active=1 AND m.active=1 AND m.subject=json_extract(?1,'$.subject') AND (json_extract(?1,'$.after') IS NULL OR o.organization_id COLLATE BINARY > json_extract(?1,'$.after')) ORDER BY o.organization_id COLLATE BINARY LIMIT json_extract(?1,'$.limit')+1), visible AS (SELECT * FROM page ORDER BY organization_id COLLATE BINARY LIMIT json_extract(?1,'$.limit')) SELECT NULL AS error,json_object('items',json((SELECT json_group_array(json_object('organization_id',organization_id,'name',name,'slug',slug,'active',json('true'),'revision',CAST(revision AS TEXT))) FROM visible)),'next_cursor',CASE WHEN (SELECT count(*) FROM page)>json_extract(?1,'$.limit') THEN (SELECT organization_id FROM visible ORDER BY organization_id COLLATE BINARY DESC LIMIT 1) END) AS response";
