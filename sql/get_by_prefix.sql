SELECT *
FROM `api_tokens`
WHERE `prefix` = ?1
  AND `token` = ?2