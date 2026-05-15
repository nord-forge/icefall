#[cfg(test)]
mod team_isolation {
    use std::sync::Arc;

    use sqlx::sqlite::SqlitePoolOptions;

    use crate::db::encryption::Encryptor;
    use crate::db::models::*;
    use crate::db::sqlite::SqliteDatabase;
    use crate::db::Database;

    async fn setup_db() -> SqliteDatabase {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect to in-memory SQLite");

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable foreign keys");

        sqlx::migrate!("src/db/migrations")
            .run(&pool)
            .await
            .expect("run migrations");

        let encryptor = Arc::new(Encryptor::new(&Encryptor::generate_key()));
        SqliteDatabase::new_with_pool(pool, encryptor)
    }

    async fn create_user(db: &SqliteDatabase, email: &str) -> User {
        db.create_user(&NewUser {
            email: email.to_string(),
            password_hash: "$argon2id$test".to_string(),
            role: "admin".to_string(),
        })
        .await
        .expect("create user")
    }

    async fn create_team_for_user(db: &SqliteDatabase, name: &str, owner: &User) -> Team {
        db.create_team(&NewTeam {
            name: name.to_string(),
            slug: name.to_lowercase().replace(' ', "-"),
            owner_id: owner.id.clone(),
        })
        .await
        .expect("create team")
    }

    #[tokio::test]
    async fn teams_are_isolated_for_apps() {
        let db = setup_db().await;
        let user_a = create_user(&db, "alice@example.com").await;
        let user_b = create_user(&db, "bob@example.com").await;
        let team_x = create_team_for_user(&db, "Team X", &user_a).await;
        let team_y = create_team_for_user(&db, "Team Y", &user_b).await;

        let app_x = db
            .create_app(&NewApp {
                name: "app-x".to_string(),
                git_repo: None,
                git_branch: "main".to_string(),
                framework: None,
                image_ref: None,
                compose_content: None,
                deploy_mode: None,
                server_id: None,
            })
            .await
            .expect("create app for team X");
        db.set_app_team(&app_x.id, &team_x.id)
            .await
            .expect("assign app to team X");

        let app_y = db
            .create_app(&NewApp {
                name: "app-y".to_string(),
                git_repo: None,
                git_branch: "main".to_string(),
                framework: None,
                image_ref: None,
                compose_content: None,
                deploy_mode: None,
                server_id: None,
            })
            .await
            .expect("create app for team Y");
        db.set_app_team(&app_y.id, &team_y.id)
            .await
            .expect("assign app to team Y");

        let apps_x = db
            .list_apps_by_team(&team_x.id)
            .await
            .expect("list apps for team X");
        let apps_y = db
            .list_apps_by_team(&team_y.id)
            .await
            .expect("list apps for team Y");

        assert_eq!(apps_x.len(), 1);
        assert_eq!(apps_x[0].name, "app-x");
        assert_eq!(apps_y.len(), 1);
        assert_eq!(apps_y[0].name, "app-y");
    }

    #[tokio::test]
    async fn teams_are_isolated_for_projects() {
        let db = setup_db().await;
        let user_a = create_user(&db, "alice@example.com").await;
        let user_b = create_user(&db, "bob@example.com").await;
        let team_x = create_team_for_user(&db, "Team X", &user_a).await;
        let team_y = create_team_for_user(&db, "Team Y", &user_b).await;

        let proj_x = db
            .create_project(&NewProject {
                name: "Project X".to_string(),
                description: None,
                color: None,
            })
            .await
            .expect("create project for team X");
        db.set_project_team(&proj_x.id, &team_x.id)
            .await
            .expect("assign project");

        let proj_y = db
            .create_project(&NewProject {
                name: "Project Y".to_string(),
                description: None,
                color: None,
            })
            .await
            .expect("create project for team Y");
        db.set_project_team(&proj_y.id, &team_y.id)
            .await
            .expect("assign project");

        let projects_x = db.list_projects_by_team(&team_x.id).await.expect("list");
        let projects_y = db.list_projects_by_team(&team_y.id).await.expect("list");

        assert_eq!(projects_x.len(), 1);
        assert_eq!(projects_x[0].name, "Project X");
        assert_eq!(projects_y.len(), 1);
        assert_eq!(projects_y[0].name, "Project Y");
    }

    #[tokio::test]
    async fn team_membership_controls_access() {
        let db = setup_db().await;
        let owner = create_user(&db, "owner@example.com").await;
        let member = create_user(&db, "member@example.com").await;
        let outsider = create_user(&db, "outsider@example.com").await;

        let team = create_team_for_user(&db, "My Team", &owner).await;

        db.add_team_member(&team.id, &member.id, "member", Some(&owner.id))
            .await
            .expect("add member");

        let owner_teams = db.list_teams_for_user(&owner.id).await.expect("list");
        let member_teams = db.list_teams_for_user(&member.id).await.expect("list");
        let outsider_teams = db.list_teams_for_user(&outsider.id).await.expect("list");

        assert_eq!(owner_teams.len(), 1);
        assert_eq!(member_teams.len(), 1);
        assert_eq!(outsider_teams.len(), 0);
    }

    #[tokio::test]
    async fn owner_auto_created_as_member() {
        let db = setup_db().await;
        let owner = create_user(&db, "owner@example.com").await;
        let team = create_team_for_user(&db, "Auto Team", &owner).await;

        let membership = db
            .get_team_membership(&team.id, &owner.id)
            .await
            .expect("get membership");

        assert!(membership.is_some());
        assert_eq!(membership.unwrap().role, "owner");
    }

    #[tokio::test]
    async fn cannot_add_duplicate_member() {
        let db = setup_db().await;
        let owner = create_user(&db, "owner@example.com").await;
        let member = create_user(&db, "member@example.com").await;
        let team = create_team_for_user(&db, "Team", &owner).await;

        db.add_team_member(&team.id, &member.id, "member", None)
            .await
            .expect("first add");

        let result = db
            .add_team_member(&team.id, &member.id, "admin", None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn member_removal_revokes_access() {
        let db = setup_db().await;
        let owner = create_user(&db, "owner@example.com").await;
        let member = create_user(&db, "member@example.com").await;
        let team = create_team_for_user(&db, "Team", &owner).await;

        db.add_team_member(&team.id, &member.id, "member", None)
            .await
            .expect("add");

        assert_eq!(db.list_teams_for_user(&member.id).await.unwrap().len(), 1);

        db.remove_team_member(&team.id, &member.id)
            .await
            .expect("remove");

        assert_eq!(db.list_teams_for_user(&member.id).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn team_deletion_blocked_with_resources() {
        let db = setup_db().await;
        let owner = create_user(&db, "owner@example.com").await;
        let team = create_team_for_user(&db, "Team", &owner).await;

        let app = db
            .create_app(&NewApp {
                name: "my-app".to_string(),
                git_repo: None,
                git_branch: "main".to_string(),
                framework: None,
                image_ref: None,
                compose_content: None,
                deploy_mode: None,
                server_id: None,
            })
            .await
            .expect("create app");
        db.set_app_team(&app.id, &team.id).await.expect("assign");

        let count = db.count_team_resources(&team.id).await.expect("count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn team_deletion_allowed_when_empty() {
        let db = setup_db().await;
        let owner = create_user(&db, "owner@example.com").await;
        let team = create_team_for_user(&db, "Team", &owner).await;

        let count = db.count_team_resources(&team.id).await.expect("count");
        assert_eq!(count, 0);

        db.delete_team(&team.id).await.expect("delete empty team");
        assert!(db.get_team(&team.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn role_update_works() {
        let db = setup_db().await;
        let owner = create_user(&db, "owner@example.com").await;
        let member = create_user(&db, "member@example.com").await;
        let team = create_team_for_user(&db, "Team", &owner).await;

        db.add_team_member(&team.id, &member.id, "viewer", None)
            .await
            .expect("add");

        let m = db
            .get_team_membership(&team.id, &member.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(m.role, "viewer");

        db.update_team_member_role(&team.id, &member.id, "admin")
            .await
            .expect("update role");

        let m = db
            .get_team_membership(&team.id, &member.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(m.role, "admin");
    }

    #[tokio::test]
    async fn invitation_flow_works() {
        let db = setup_db().await;
        let owner = create_user(&db, "owner@example.com").await;
        let team = create_team_for_user(&db, "Team", &owner).await;

        let invitation = db
            .create_team_invitation(
                &team.id,
                "invited@example.com",
                "member",
                "test-token-123",
                &owner.id,
                "2099-01-01T00:00:00.000Z",
            )
            .await
            .expect("create invitation");

        assert_eq!(invitation.email, "invited@example.com");
        assert_eq!(invitation.role, "member");

        let found = db
            .get_team_invitation_by_token("test-token-123")
            .await
            .expect("lookup");
        assert!(found.is_some());

        let invitations = db.list_team_invitations(&team.id).await.expect("list");
        assert_eq!(invitations.len(), 1);

        db.delete_team_invitation(&invitation.id)
            .await
            .expect("delete");

        let found = db
            .get_team_invitation_by_token("test-token-123")
            .await
            .expect("lookup after delete");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn api_tokens_scoped_to_team() {
        let db = setup_db().await;
        let user = create_user(&db, "user@example.com").await;
        let team_a = create_team_for_user(&db, "Team A", &user).await;
        let team_b = create_team_for_user(&db, "Team B", &user).await;

        db.create_api_token(&user.id, "Token A", "hash-a", None, Some(&team_a.id))
            .await
            .expect("create token A");
        db.create_api_token(&user.id, "Token B", "hash-b", None, Some(&team_b.id))
            .await
            .expect("create token B");

        let tokens_a = db.list_api_tokens_by_team(&team_a.id).await.expect("list");
        let tokens_b = db.list_api_tokens_by_team(&team_b.id).await.expect("list");

        assert_eq!(tokens_a.len(), 1);
        assert_eq!(tokens_a[0].name, "Token A");
        assert_eq!(tokens_b.len(), 1);
        assert_eq!(tokens_b[0].name, "Token B");
    }

    #[tokio::test]
    async fn cross_team_server_sharing() {
        let db = setup_db().await;
        let user_a = create_user(&db, "alice@example.com").await;
        let user_b = create_user(&db, "bob@example.com").await;
        let team_x = create_team_for_user(&db, "Team X", &user_a).await;
        let team_y = create_team_for_user(&db, "Team Y", &user_b).await;

        let server = db
            .create_server(&NewServer {
                name: "worker-1".to_string(),
                host: "10.0.0.1".to_string(),
                role: "worker".to_string(),
                token_hash: None,
                labels: None,
                resources: None,
                public_key: None,
            })
            .await
            .expect("create server");

        let share = db
            .share_server_with_team(&server.id, &team_y.id, "deploy", &user_a.id)
            .await
            .expect("share server");
        assert_eq!(share.access_level, "deploy");

        let shares = db
            .list_server_shares(&server.id)
            .await
            .expect("list shares");
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].team_id, team_y.id);

        let shared = db
            .list_servers_shared_with_team(&team_y.id)
            .await
            .expect("list shared");
        assert_eq!(shared.len(), 1);
        assert_eq!(shared[0].0.name, "worker-1");
        assert_eq!(shared[0].1, "deploy");

        let not_shared = db
            .list_servers_shared_with_team(&team_x.id)
            .await
            .expect("list not shared");
        assert_eq!(not_shared.len(), 0);
    }

    #[tokio::test]
    async fn revoke_server_share() {
        let db = setup_db().await;
        let user_a = create_user(&db, "alice@example.com").await;
        let user_b = create_user(&db, "bob@example.com").await;
        let _team_x = create_team_for_user(&db, "Team X", &user_a).await;
        let team_y = create_team_for_user(&db, "Team Y", &user_b).await;

        let server = db
            .create_server(&NewServer {
                name: "worker-2".to_string(),
                host: "10.0.0.2".to_string(),
                role: "worker".to_string(),
                token_hash: None,
                labels: None,
                resources: None,
                public_key: None,
            })
            .await
            .expect("create server");

        db.share_server_with_team(&server.id, &team_y.id, "read-only", &user_a.id)
            .await
            .expect("share");

        db.revoke_server_share(&server.id, &team_y.id)
            .await
            .expect("revoke");

        let shares = db.list_server_shares(&server.id).await.expect("list");
        assert_eq!(shares.len(), 0);
    }

    #[tokio::test]
    async fn session_team_context() {
        let db = setup_db().await;
        let user = create_user(&db, "user@example.com").await;
        let team = create_team_for_user(&db, "Team", &user).await;

        let session = db
            .create_session(&user.id, "2099-01-01T00:00:00.000Z")
            .await
            .expect("create session");

        assert!(session.active_team_id.is_none());

        db.set_session_team(&session.id, &team.id)
            .await
            .expect("set team");

        let updated = db.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(updated.active_team_id, Some(team.id));
    }

    #[tokio::test]
    async fn team_slug_must_be_unique() {
        let db = setup_db().await;
        let user = create_user(&db, "user@example.com").await;

        create_team_for_user(&db, "My Team", &user).await;

        let result = db
            .create_team(&NewTeam {
                name: "My Team 2".to_string(),
                slug: "my-team".to_string(),
                owner_id: user.id.clone(),
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn user_can_be_in_multiple_teams() {
        let db = setup_db().await;
        let user = create_user(&db, "user@example.com").await;

        create_team_for_user(&db, "Team 1", &user).await;
        create_team_for_user(&db, "Team 2", &user).await;
        create_team_for_user(&db, "Team 3", &user).await;

        let teams = db.list_teams_for_user(&user.id).await.expect("list");
        assert_eq!(teams.len(), 3);
    }

    #[tokio::test]
    async fn list_team_members_shows_emails() {
        let db = setup_db().await;
        let owner = create_user(&db, "owner@example.com").await;
        let member1 = create_user(&db, "m1@example.com").await;
        let member2 = create_user(&db, "m2@example.com").await;
        let team = create_team_for_user(&db, "Team", &owner).await;

        db.add_team_member(&team.id, &member1.id, "admin", Some(&owner.id))
            .await
            .expect("add m1");
        db.add_team_member(&team.id, &member2.id, "viewer", Some(&owner.id))
            .await
            .expect("add m2");

        let members = db.list_team_members(&team.id).await.expect("list");
        assert_eq!(members.len(), 3);

        let emails: Vec<&str> = members.iter().map(|m| m.email.as_str()).collect();
        assert!(emails.contains(&"owner@example.com"));
        assert!(emails.contains(&"m1@example.com"));
        assert!(emails.contains(&"m2@example.com"));
    }
}
