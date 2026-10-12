-- Promote a specific account to `admin`.
--
-- This is deliberately a data migration keyed by user ID, not application
-- logic: the code has no notion of privileged accounts. The statement is
-- idempotent — it changes nothing if the user already has the role, and a
-- database that does not contain this account (e.g. a fresh deployment) is
-- left untouched.
UPDATE users
SET role = 'admin'
WHERE id = '6e23d37e-df9b-499a-adce-ae5e8ae28e32'
  AND role <> 'admin';