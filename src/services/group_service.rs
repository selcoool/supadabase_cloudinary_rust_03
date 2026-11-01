use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    models::group::{ChatGroup, CreateGroup},
    repositories::group_repository,
};

// =====================================
// ADD GROUP MEMBERS
// =====================================

pub async fn add_group_members(
    pool: &PgPool,
    group_id: i64,
    user_ids: Vec<Uuid>,
) -> Result<u64, String> {
    if user_ids.is_empty() {
        return Err("Danh sách user_ids không được rỗng".into());
    }

    if group_id <= 0 {
        return Err("group_id phải lớn hơn 0".into());
    }

    if user_ids.len() > 100 {
        return Err("Chỉ được thêm tối đa 100 người mỗi lần".into());
    }

    // Kiểm tra group có tồn tại không
    let group_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM chat_groups WHERE id = $1)")
            .bind(group_id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;

    if !group_exists {
        return Err("Group không tồn tại".into());
    }

    // Đếm số user_id không trùng lặp
    let unique_ids: std::collections::HashSet<Uuid> = user_ids.iter().copied().collect();

    let unique_count = unique_ids.len();

    // Kiểm tra user có tồn tại không
    let unique_user_ids: Vec<Uuid> = unique_ids.into_iter().collect();

    let existing_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM users
        WHERE id = ANY($1::UUID[])
        "#,
    )
    .bind(&unique_user_ids)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    if existing_count != unique_count as i64 {
        return Err("Có user_id không tồn tại".into());
    }

    // Thêm thành viên
    let added = group_repository::add_members(pool, group_id, &unique_user_ids)
        .await
        .map_err(|e| e.to_string())?;

    Ok(added)
}

// =====================================
// CREATE GROUP
// =====================================

pub async fn create_group(pool: &PgPool, input: CreateGroup) -> Result<ChatGroup, String> {
    let name = input.name.trim();

    if name.is_empty() {
        return Err("Tên nhóm không được để trống".into());
    }

    if name.len() > 100 {
        return Err("Tên nhóm tối đa 100 ký tự".into());
    }

    group_repository::create_group(pool, name)
        .await
        .map_err(|e| e.to_string())
}

// use sqlx::PgPool;

// use crate::{
//     models::group::{ChatGroup, CreateGroup},
//     repositories::group_repository,
// };

// pub async fn add_group_members(
//     pool: &PgPool,
//     group_id: i64,
//     user_ids: Vec<i64>,
// ) -> Result<u64, String> {
//     if user_ids.is_empty() {
//         return Err("Danh sách user_ids không được rỗng".to_string());
//     }

//     if user_ids.iter().any(|id| *id <= 0) {
//         return Err("user_id phải lớn hơn 0".to_string());
//     }

//     if user_ids.len() > 100 {
//         return Err("Chỉ được thêm tối đa 100 người mỗi lần".to_string());
//     }

//     // Kiểm tra group
//     let group_exists =
//         sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM chat_groups WHERE id = $1)")
//             .bind(group_id)
//             .fetch_one(pool)
//             .await
//             .map_err(|e| e.to_string())?;

//     dbg!(&group_exists);

//     if !group_exists {
//         return Err("Group không tồn tại".to_string());
//     }

//     // Kiểm tra tất cả user có tồn tại không
//     let existing_count = sqlx::query_scalar::<_, i64>(
//         r#"
//         SELECT COUNT(DISTINCT id)
//         FROM users
//         WHERE id = ANY($1::BIGINT[])
//         "#,
//     )
//     .bind(&user_ids)
//     .fetch_one(pool)
//     .await
//     .map_err(|e| e.to_string())?;

//     let unique_count = user_ids
//         .iter()
//         .collect::<std::collections::HashSet<_>>()
//         .len();

//     // let unique_count = [1, 2, 3, 4, 5]
//     //     .iter()
//     //     .collect::<std::collections::HashSet<_>>()
//     //     .len();
//     dbg!(&unique_count);
//     if existing_count != unique_count as i64 {
//         return Err("Có user_id không tồn tại".to_string());
//     }

//     // Thêm thành viên
//     let added = group_repository::add_members(pool, group_id, &user_ids)
//         .await
//         .map_err(|e| e.to_string())?;

//     Ok(added)
// }

// pub async fn create_group(pool: &PgPool, input: CreateGroup) -> Result<ChatGroup, String> {
//     let name = input.name.trim();

//     if name.is_empty() {
//         return Err("Tên nhóm không được để trống".into());
//     }

//     if name.len() > 100 {
//         return Err("Tên nhóm tối đa 100 ký tự".into());
//     }

//     group_repository::create_group(pool, name)
//         .await
//         .map_err(|e| e.to_string())
// }
