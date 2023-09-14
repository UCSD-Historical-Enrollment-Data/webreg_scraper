CREATE TABLE IF NOT EXISTS `api_tokens` (
    `prefix` VARCHAR(255) NOT NULL,
    `token` VARCHAR(255) NOT NULL PRIMARY KEY UNIQUE,
    `created_at` DATETIME NOT NULL,
    `expires_at` DATETIME NOT NULL,
    `description` TEXT
)