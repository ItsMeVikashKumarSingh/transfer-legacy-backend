use crate::tests::test_utils::spawn_app;

#[tokio::test]
async fn prune_target_user() {
    let ctx = spawn_app().await;
    let email = "vikashbro111@gmail.com";
    
    // Find the user ID in auth.users using text cast to avoid uuid/text mismatches
    let user: Option<(uuid::Uuid,)> = sqlx::query_as("SELECT id FROM auth.users WHERE email::text = $1::text")
        .bind(email)
        .fetch_optional(&ctx.db)
        .await
        .unwrap();
        
    if let Some((user_id,)) = user {
        println!("🚀 [PRUNE_USER] Found user '{}' with UUID: {}", email, user_id);
        
        // 1. Delete from public.users first due to foreign key constraints
        let del_public = sqlx::query("DELETE FROM public.users WHERE id::text = $1::text")
            .bind(user_id.to_string())
            .execute(&ctx.db)
            .await
            .unwrap();
        println!("🚀 [PRUNE_USER] Deleted from public.users: {} row(s)", del_public.rows_affected());
        
        // 2. Delete from auth.users
        let del_auth = sqlx::query("DELETE FROM auth.users WHERE id::text = $1::text")
            .bind(user_id.to_string())
            .execute(&ctx.db)
            .await
            .unwrap();
        println!("🚀 [PRUNE_USER] Deleted from auth.users: {} row(s)", del_auth.rows_affected());
        
        println!("🚀 [PRUNE_USER] Successfully pruned user '{}' from both public.users and auth.users databases!", email);
    } else {
        println!("🚀 [PRUNE_USER] User '{}' not found in auth.users. Nothing to prune.", email);
    }
}
