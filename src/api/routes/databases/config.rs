use std::collections::HashMap;

pub(super) struct DbTypeConfig {
    pub image: &'static str,
    pub port: u16,
    pub env_vars: fn(&str, &str, &str) -> Vec<String>,
    pub connection_string: fn(&str, &str, &str, &str) -> String,
    pub default_memory_mb: i64,
    pub env_var_name: &'static str,
}

pub(super) fn db_configs() -> HashMap<&'static str, DbTypeConfig> {
    let mut m = HashMap::new();
    m.insert(
        "postgres",
        DbTypeConfig {
            image: "postgres:17",
            port: 5432,
            env_vars: |user, pass, _db| {
                vec![
                    format!("POSTGRES_USER={user}"),
                    format!("POSTGRES_PASSWORD={pass}"),
                    format!("POSTGRES_DB={user}"),
                ]
            },
            connection_string: |container, _port, user, pass| {
                format!("postgresql://{user}:{pass}@{container}:5432/{user}")
            },
            default_memory_mb: 1024,
            env_var_name: "DATABASE_URL",
        },
    );
    m.insert(
        "mysql",
        DbTypeConfig {
            image: "mysql:8",
            port: 3306,
            env_vars: |user, pass, db| {
                vec![
                    format!("MYSQL_USER={user}"),
                    format!("MYSQL_PASSWORD={pass}"),
                    format!("MYSQL_DATABASE={db}"),
                    format!("MYSQL_ROOT_PASSWORD={pass}"),
                ]
            },
            connection_string: |container, _port, user, pass| {
                format!("mysql://{user}:{pass}@{container}:3306/{user}")
            },
            default_memory_mb: 1024,
            env_var_name: "DATABASE_URL",
        },
    );
    m.insert(
        "redis",
        DbTypeConfig {
            image: "redis:7",
            port: 6379,
            env_vars: |_user, pass, _db| vec![format!("REDIS_PASSWORD={pass}")],
            connection_string: |container, _port, _user, pass| {
                format!("redis://:{pass}@{container}:6379")
            },
            default_memory_mb: 256,
            env_var_name: "REDIS_URL",
        },
    );
    m.insert(
        "mongo",
        DbTypeConfig {
            image: "mongo:7",
            port: 27017,
            env_vars: |user, pass, db| {
                vec![
                    format!("MONGO_INITDB_ROOT_USERNAME={user}"),
                    format!("MONGO_INITDB_ROOT_PASSWORD={pass}"),
                    format!("MONGO_INITDB_DATABASE={db}"),
                ]
            },
            connection_string: |container, _port, user, pass| {
                format!("mongodb://{user}:{pass}@{container}:27017/{user}")
            },
            default_memory_mb: 512,
            env_var_name: "MONGODB_URL",
        },
    );
    m.insert(
        "mariadb",
        DbTypeConfig {
            image: "mariadb:11",
            port: 3306,
            env_vars: |user, pass, db| {
                vec![
                    format!("MARIADB_USER={user}"),
                    format!("MARIADB_PASSWORD={pass}"),
                    format!("MARIADB_DATABASE={db}"),
                    format!("MARIADB_ROOT_PASSWORD={pass}"),
                ]
            },
            connection_string: |container, _port, user, pass| {
                format!("mysql://{user}:{pass}@{container}:3306/{user}")
            },
            default_memory_mb: 1024,
            env_var_name: "DATABASE_URL",
        },
    );
    m.insert(
        "clickhouse",
        DbTypeConfig {
            image: "clickhouse/clickhouse-server:24",
            port: 8123,
            env_vars: |user, pass, db| {
                vec![
                    format!("CLICKHOUSE_USER={user}"),
                    format!("CLICKHOUSE_PASSWORD={pass}"),
                    format!("CLICKHOUSE_DB={db}"),
                    "CLICKHOUSE_DEFAULT_ACCESS_MANAGEMENT=1".to_string(),
                ]
            },
            connection_string: |container, _port, user, pass| {
                format!("clickhouse://{user}:{pass}@{container}:8123/{user}")
            },
            default_memory_mb: 2048,
            env_var_name: "CLICKHOUSE_URL",
        },
    );
    m.insert(
        "keydb",
        DbTypeConfig {
            image: "eqalpha/keydb:latest",
            port: 6379,
            env_vars: |_user, pass, _db| vec![format!("REDIS_PASSWORD={pass}")],
            connection_string: |container, _port, _user, pass| {
                format!("redis://:{pass}@{container}:6379")
            },
            default_memory_mb: 256,
            env_var_name: "REDIS_URL",
        },
    );
    m.insert(
        "dragonfly",
        DbTypeConfig {
            image: "docker.dragonflydb.io/dragonflydb/dragonfly:latest",
            port: 6379,
            env_vars: |_user, pass, _db| vec![format!("REDIS_PASSWORD={pass}")],
            connection_string: |container, _port, _user, pass| {
                format!("redis://:{pass}@{container}:6379")
            },
            default_memory_mb: 512,
            env_var_name: "REDIS_URL",
        },
    );
    m.insert(
        "valkey",
        DbTypeConfig {
            image: "valkey/valkey:8",
            port: 6379,
            env_vars: |_user, pass, _db| vec![format!("REDIS_PASSWORD={pass}")],
            connection_string: |container, _port, _user, pass| {
                format!("redis://:{pass}@{container}:6379")
            },
            default_memory_mb: 256,
            env_var_name: "REDIS_URL",
        },
    );
    m.insert(
        "cockroachdb",
        DbTypeConfig {
            image: "cockroachdb/cockroach:latest",
            port: 26257,
            env_vars: |_user, _pass, _db| vec![],
            connection_string: |container, _port, _user, _pass| {
                format!("postgresql://root@{container}:26257/defaultdb?sslmode=disable")
            },
            default_memory_mb: 2048,
            env_var_name: "DATABASE_URL",
        },
    );
    m.insert(
        "cassandra",
        DbTypeConfig {
            image: "cassandra:5",
            port: 9042,
            env_vars: |_user, _pass, db| {
                vec![
                    format!("CASSANDRA_CLUSTER_NAME={db}"),
                    "CASSANDRA_DC=dc1".to_string(),
                    "CASSANDRA_RACK=rack1".to_string(),
                ]
            },
            connection_string: |container, _port, _user, _pass| {
                format!("cassandra://{container}:9042")
            },
            default_memory_mb: 2048,
            env_var_name: "CASSANDRA_URL",
        },
    );
    m
}

pub(super) fn generate_password() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..32)
        .map(|_| {
            let idx = rng.random_range(0..62);
            match idx {
                0..=9 => (b'0' + idx) as char,
                10..=35 => (b'a' + idx - 10) as char,
                _ => (b'A' + idx - 36) as char,
            }
        })
        .collect()
}
