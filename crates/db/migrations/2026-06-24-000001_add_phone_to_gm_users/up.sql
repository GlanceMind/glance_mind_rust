-- Add a contact phone number to users.
--
-- Why: registration now REQUIRES an 11-digit mainland-China mobile number as the
-- user's contact method (validated at the API layer, ^1[3-9]\d{9}$), and the admin
-- panel displays it. The column is NULLable because users created before this change
-- have no phone on file; the NOT-NULL-at-registration guarantee is enforced by the
-- application (UserRegisterDto), not the DB, so existing rows stay valid.
--
-- VARCHAR(20) (not 11) leaves head-room for formatting/country-code without a future
-- migration; the validator still pins new values to exactly 11 digits.
--
-- Shared DB: the admin service (glance_mind_admin) reads gm_users directly via Diesel,
-- so this single column is what surfaces the phone in both the API response and admin.

ALTER TABLE gm_users
    ADD COLUMN phone VARCHAR(20);
