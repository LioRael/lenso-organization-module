use super::*;
use admin::OrganizationAdminProvider;
use directory::OrganizationDirectoryProvider;
use lenso_kernel::CancellationToken;
use members::OrganizationMembershipAdminProvider;
use membership::OrganizationMembershipProvider;
use std::cell::RefCell;

#[derive(Clone, Debug)]
struct Sqlite(Rc<RefCell<rusqlite::Connection>>);
impl Sqlite {
    fn new() -> Self {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        Self(Rc::new(RefCell::new(db)))
    }
}
impl Transport for Sqlite {
    fn batch(
        &self,
        statements: Vec<Statement>,
    ) -> LocalBoxFuture<'_, Result<Vec<Vec<Value>>, Error>> {
        Box::pin(async move {
            let mut db = self.0.borrow_mut();
            let tx = db.transaction().map_err(|_| Error::Transport)?;
            let mut results = vec![];
            for stmt in statements {
                let mut query = tx.prepare(&stmt.sql).map_err(|_| Error::Transport)?;
                let columns = query
                    .column_names()
                    .iter()
                    .map(|v| (*v).to_owned())
                    .collect::<Vec<_>>();
                let params = stmt
                    .params
                    .iter()
                    .map(|v| match v {
                        Value::Null => rusqlite::types::Value::Null,
                        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                        Value::Number(n) => rusqlite::types::Value::Integer(n.as_i64().unwrap()),
                        _ => panic!("unsupported binding"),
                    })
                    .collect::<Vec<_>>();
                let mut rows = query
                    .query(rusqlite::params_from_iter(params))
                    .map_err(|_| Error::Transport)?;
                let mut values = vec![];
                while let Some(row) = rows.next().map_err(|_| Error::Transport)? {
                    let mut obj = serde_json::Map::new();
                    for (i, key) in columns.iter().enumerate() {
                        let v = match row.get_ref(i).unwrap() {
                            rusqlite::types::ValueRef::Null => Value::Null,
                            rusqlite::types::ValueRef::Integer(n) => n.into(),
                            rusqlite::types::ValueRef::Text(s) => {
                                String::from_utf8(s.to_vec()).unwrap().into()
                            }
                            _ => panic!("unexpected SQL value"),
                        };
                        obj.insert(key.clone(), v);
                    }
                    values.push(Value::Object(obj));
                }
                results.push(values);
            }
            tx.commit().map_err(|_| Error::Transport)?;
            Ok(results)
        })
    }
}

fn context(caller: &str) -> InvocationContext {
    InvocationContext::new(1, None, CancellationToken::new()).with_caller_instance(caller)
}
async fn prepared() -> (Provider, Sqlite) {
    let db = Sqlite::new();
    schema::plan().unwrap().setup(&db).await.unwrap();
    schema::plan().unwrap().verify(&db).await.unwrap();
    let provider = Provider {
        config: D1Config {
            binding: "ORGANIZATION".into(),
            admin_callers: vec!["admin:one".into(), "admin:two".into()],
            directory_callers: vec!["directory".into()],
            membership_admin_callers: vec!["members".into()],
        },
        binding: EventBinding(Rc::new(db.clone())),
        prepared: Rc::new(Cell::new(true)),
    };
    (provider, db)
}
fn create_request() -> admin::CreateOrganizationRequest {
    admin::CreateOrganizationRequest {
        idempotency_key: "create".into(),
        name: "  Acme  ".into(),
        slug: "acme".into(),
        owner_subject: "owner".into(),
    }
}
async fn create(p: &Provider) -> admin::CreateOrganizationResponse {
    p.create_organization(context("admin:one"), create_request())
        .await
        .unwrap()
        .unwrap()
}
fn add_request(org: &str, key: &str, subject: &str) -> members::AddMemberRequest {
    members::AddMemberRequest {
        organization_id: org.into(),
        idempotency_key: key.into(),
        subject: subject.into(),
    }
}
fn remove_request(org: &str, key: &str, subject: &str) -> members::RemoveMemberRequest {
    members::RemoveMemberRequest {
        organization_id: org.into(),
        idempotency_key: key.into(),
        subject: subject.into(),
    }
}
fn count(db: &Sqlite, table: &str) -> i64 {
    db.0.borrow()
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn exact_admission_is_inside_every_write_batch_including_replays() {
    futures::executor::block_on(async {
        let (p, db) = prepared().await;
        for caller in [
            None,
            Some("admin"),
            Some("admin:one:child"),
            Some("Admin:one"),
            Some("admin:one "),
            Some("members"),
        ] {
            let result: Result<admin::CreateOrganizationResponse, admin::CreateOrganizationError> =
                storage::mutate(
                    &db,
                    storage::Mutation::Create,
                    caller,
                    &p.config.admin_callers,
                    &create_request(),
                    None,
                )
                .await
                .unwrap();
            assert_eq!(result, Err(admin::CreateOrganizationError::Forbidden));
            assert_eq!(count(&db, "organizations"), 0);
            assert_eq!(count(&db, "organization_receipts"), 0);
        }
        let org = create(&p).await;
        let denied: Result<admin::CreateOrganizationResponse, admin::CreateOrganizationError> =
            storage::mutate(
                &db,
                storage::Mutation::Create,
                Some("admin:one"),
                &[],
                &create_request(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(denied, Err(admin::CreateOrganizationError::Forbidden));
        let added = p
            .add_member(
                context("members"),
                add_request(&org.organization_id, "add", "member"),
            )
            .await
            .unwrap()
            .unwrap();
        for caller in [
            None,
            Some("Members"),
            Some("members:other"),
            Some("admin:one"),
        ] {
            let added: Result<members::AddMemberResponse, members::AddMemberError> =
                storage::mutate(
                    &db,
                    storage::Mutation::Add,
                    caller,
                    &p.config.membership_admin_callers,
                    &add_request(&org.organization_id, "add-deny", "other"),
                    None,
                )
                .await
                .unwrap();
            assert_eq!(added, Err(members::AddMemberError::Forbidden));
            let removed: Result<members::RemoveMemberResponse, members::RemoveMemberError> =
                storage::mutate(
                    &db,
                    storage::Mutation::Remove,
                    caller,
                    &p.config.membership_admin_callers,
                    &remove_request(&org.organization_id, "remove-deny", "member"),
                    None,
                )
                .await
                .unwrap();
            assert_eq!(removed, Err(members::RemoveMemberError::Forbidden));
        }
        assert_eq!(added.revision, "1");
        assert_eq!(count(&db, "organization_memberships"), 2);
        assert_eq!(count(&db, "organization_receipts"), 2);
        assert_eq!(count(&db, "organization_operation"), 0);
    });
}

#[test]
#[allow(clippy::too_many_lines)]
fn creation_and_membership_preserve_normalized_intents_replays_and_owner_protection() {
    futures::executor::block_on(async {
        let (p, db) = prepared().await;
        let first = create(&p).await;
        assert!(first.created);
        let mut request = create_request();
        request.name = "Acme".into();
        let replay = p
            .create_organization(context("admin:one"), request.clone())
            .await
            .unwrap()
            .unwrap();
        assert!(!replay.created);
        assert_eq!(replay.organization_id, first.organization_id);
        assert_eq!(replay.owner_membership_id, first.owner_membership_id);
        request.owner_subject = "different".into();
        assert_eq!(
            p.create_organization(context("admin:one"), request.clone())
                .await
                .unwrap(),
            Err(admin::CreateOrganizationError::IdempotencyConflict)
        );
        assert_eq!(
            p.create_organization(context("admin:two"), request.clone())
                .await
                .unwrap(),
            Err(admin::CreateOrganizationError::SlugConflict)
        );
        request.slug = "second".into();
        assert!(
            p.create_organization(context("admin:two"), request)
                .await
                .unwrap()
                .unwrap()
                .created
        );
        let org = &first.organization_id;
        assert_eq!(
            p.remove_member(context("members"), remove_request(org, "owner", "owner"))
                .await
                .unwrap(),
            Err(members::RemoveMemberError::OwnerProtected)
        );
        assert_eq!(
            p.remove_member(
                context("members"),
                remove_request(org, "missing", "missing")
            )
            .await
            .unwrap(),
            Err(members::RemoveMemberError::MembershipNotFound)
        );
        assert_eq!(
            p.add_member(
                context("members"),
                add_request("absent", "absent", "member")
            )
            .await
            .unwrap(),
            Err(members::AddMemberError::OrganizationNotFound)
        );
        let member = p
            .add_member(context("members"), add_request(org, "add", "member"))
            .await
            .unwrap()
            .unwrap();
        assert!(member.created);
        assert_eq!(member.revision, "1");
        let same = p
            .add_member(context("members"), add_request(org, "other-key", "member"))
            .await
            .unwrap()
            .unwrap();
        assert!(!same.created);
        assert_eq!(same.membership_id, member.membership_id);
        assert_eq!(
            p.remove_member(context("members"), remove_request(org, "add", "member"))
                .await
                .unwrap(),
            Err(members::RemoveMemberError::IdempotencyConflict)
        );
        let removed = p
            .remove_member(context("members"), remove_request(org, "remove", "member"))
            .await
            .unwrap()
            .unwrap();
        assert!(removed.removed);
        assert_eq!(removed.revision, "2");
        let replay = p
            .remove_member(context("members"), remove_request(org, "remove", "member"))
            .await
            .unwrap()
            .unwrap();
        assert!(!replay.removed);
        assert_eq!(replay.revision, "2");
        let replay = p
            .add_member(context("members"), add_request(org, "add", "member"))
            .await
            .unwrap()
            .unwrap();
        assert!(!replay.created);
        assert_eq!(replay.membership_id, member.membership_id);
        assert_eq!(replay.revision, "1");
        let active = p
            .check_membership(
                context("any-peer"),
                membership::CheckMembershipRequest {
                    organization_id: org.clone(),
                    subject: "member".into(),
                },
            )
            .await
            .unwrap()
            .unwrap();
        assert!(!active.active);
        assert!(!active.owner);
        let readded = p
            .add_member(context("members"), add_request(org, "readd", "member"))
            .await
            .unwrap()
            .unwrap();
        assert!(readded.created);
        assert_ne!(readded.membership_id, member.membership_id);
        assert_eq!(count(&db, "organization_operation"), 0);
    });
}

#[test]
fn late_failure_rolls_back_owner_membership_receipts_and_operation_workspace() {
    futures::executor::block_on(async {
        let (p, db) = prepared().await;
        db.0.borrow().execute_batch("CREATE TRIGGER reject_receipt BEFORE INSERT ON organization_receipts BEGIN SELECT RAISE(ABORT,'private synthetic failure'); END;").unwrap();
        let err = p
            .create_organization(context("admin:one"), create_request())
            .await
            .unwrap_err();
        assert!(!format!("{err:?}").contains("private synthetic"));
        for table in [
            "organizations",
            "organization_memberships",
            "organization_receipts",
            "organization_operation",
        ] {
            assert_eq!(count(&db, table), 0);
        }
        db.0.borrow()
            .execute_batch("DROP TRIGGER reject_receipt")
            .unwrap();
        let org = create(&p).await;
        p.add_member(
            context("members"),
            add_request(&org.organization_id, "member", "member"),
        )
        .await
        .unwrap()
        .unwrap();
        db.0.borrow().execute_batch("CREATE TRIGGER reject_receipt BEFORE INSERT ON organization_receipts BEGIN SELECT RAISE(ABORT,'private synthetic failure'); END;").unwrap();
        assert!(
            p.add_member(
                context("members"),
                add_request(&org.organization_id, "new", "new")
            )
            .await
            .is_err()
        );
        assert!(
            p.remove_member(
                context("members"),
                remove_request(&org.organization_id, "remove", "member")
            )
            .await
            .is_err()
        );
        assert_eq!(count(&db, "organization_memberships"), 2);
        assert_eq!(count(&db, "organization_receipts"), 2);
        assert_eq!(count(&db, "organization_operation"), 0);
        let active = p
            .check_membership(
                context("peer"),
                membership::CheckMembershipRequest {
                    organization_id: org.organization_id,
                    subject: "member".into(),
                },
            )
            .await
            .unwrap()
            .unwrap();
        assert!(active.active);
    });
}

#[test]
fn revisions_remain_decimal_text_and_overflow_fails_closed() {
    futures::executor::block_on(async {
        let (p, db) = prepared().await;
        let org = create(&p).await;
        p.add_member(
            context("members"),
            add_request(&org.organization_id, "a", "a"),
        )
        .await
        .unwrap()
        .unwrap();
        db.0.borrow()
            .execute_batch(
                "UPDATE organization_memberships SET revision=9007199254740993 WHERE subject='a'",
            )
            .unwrap();
        let removed = p
            .remove_member(
                context("members"),
                remove_request(&org.organization_id, "r", "a"),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(removed.revision, "9007199254740994");
        p.add_member(
            context("members"),
            add_request(&org.organization_id, "b", "b"),
        )
        .await
        .unwrap()
        .unwrap();
        db.0.borrow().execute_batch("UPDATE organization_memberships SET revision=9223372036854775807 WHERE subject='b'").unwrap();
        assert!(
            p.remove_member(
                context("members"),
                remove_request(&org.organization_id, "overflow", "b")
            )
            .await
            .is_err()
        );
        assert_eq!(
            db.0.borrow()
                .query_row(
                    "SELECT active FROM organization_memberships WHERE subject='b'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(count(&db, "organization_operation"), 0);
    });
}

#[test]
fn current_schema_verification_is_read_only_and_rejects_history_drift() {
    futures::executor::block_on(async {
        let db = Sqlite::new();
        assert!(matches!(
            schema::plan().unwrap().verify(&db).await,
            Err(Error::SetupRequired)
        ));
        assert_eq!(count(&db, "sqlite_master"), 0);
        schema::plan().unwrap().setup(&db).await.unwrap();
        let tables = count(&db, "sqlite_master");
        schema::plan().unwrap().verify(&db).await.unwrap();
        assert_eq!(count(&db, "sqlite_master"), tables);
        db.0.borrow()
            .execute_batch("UPDATE _lenso_migrations SET checksum='drift'")
            .unwrap();
        assert!(matches!(
            schema::plan().unwrap().verify(&db).await,
            Err(Error::History)
        ));
    });
}

#[test]
#[allow(clippy::too_many_lines)]
fn bounded_sorted_pages_filters_and_directory_visibility_match_capabilities() {
    futures::executor::block_on(async {
        let (p, db) = prepared().await;
        for id in ["org_z", "org_A", "org_a"] {
            db.0.borrow()
                .execute(
                    "INSERT INTO organizations(organization_id,name,slug) VALUES(?1,?1,?1)",
                    [id],
                )
                .unwrap();
            db.0.borrow().execute("INSERT INTO organization_memberships(membership_id,organization_id,subject,is_owner) VALUES(?1,?1,'owner',1)",[id]).unwrap();
        }
        let list = admin::ListOrganizationsRequest {
            cursor: None,
            limit: 2,
            slug: None,
            status: admin::ListOrganizationsRequestStatus::All,
        };
        let first = p
            .list_organizations(context("admin:one"), list.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            first
                .organizations
                .iter()
                .map(|o| o.organization_id.as_str())
                .collect::<Vec<_>>(),
            ["org_A", "org_a"]
        );
        assert_eq!(first.next_cursor.as_deref(), Some("org_a"));
        let mut second = list;
        second.cursor = first.next_cursor;
        let second = p
            .list_organizations(context("admin:one"), second)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(second.organizations[0].organization_id, "org_z");
        assert!(second.next_cursor.is_none());
        let dir = p
            .list_for_subject(
                context("directory"),
                directory::ListForSubjectRequest {
                    subject: "owner".into(),
                    limit: 2,
                    after: None,
                },
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(dir.items.len(), 2);
        assert_eq!(dir.next_cursor.as_deref(), Some("org_a"));
        let org = p
            .get_organization(
                context("directory"),
                directory::GetOrganizationRequest {
                    organization_id: "org_A".into(),
                },
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(org.name, "org_A");
        assert_eq!(org.revision, "1");
        assert!(org.active);
        for subject in ["z", "A", "a"] {
            p.add_member(context("members"), add_request("org_A", subject, subject))
                .await
                .unwrap()
                .unwrap();
        }
        let list = members::ListMembersRequest {
            organization_id: "org_A".into(),
            cursor: None,
            limit: 2,
            status: members::ListMembersRequestStatus::All,
            subject: None,
        };
        let first = p
            .list_members(context("members"), list.clone())
            .await
            .unwrap()
            .unwrap();
        let again = p
            .list_members(context("members"), list.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first, again);
        assert!(
            first
                .members
                .windows(2)
                .all(|w| w[0].membership_id < w[1].membership_id)
        );
        let mut next = list;
        next.cursor = first.next_cursor;
        let next = p
            .list_members(context("members"), next)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(next.members.len(), 2);
        assert!(next.next_cursor.is_none());
        db.0.borrow()
            .execute_batch("UPDATE organizations SET active=0 WHERE organization_id='org_A'")
            .unwrap();
        let checked = p
            .check_membership(
                context("peer"),
                membership::CheckMembershipRequest {
                    organization_id: "org_A".into(),
                    subject: "owner".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            checked,
            Err(membership::CheckMembershipError::OrganizationNotFound)
        );
        assert!(
            !p.get_organization(
                context("directory"),
                directory::GetOrganizationRequest {
                    organization_id: "org_A".into()
                }
            )
            .await
            .unwrap()
            .unwrap()
            .active
        );
        let directory = p
            .list_for_subject(
                context("directory"),
                directory::ListForSubjectRequest {
                    subject: "owner".into(),
                    limit: 100,
                    after: None,
                },
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(directory.items.len(), 2);
    });
}
