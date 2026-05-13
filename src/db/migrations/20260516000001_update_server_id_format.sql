-- Rename the control-plane server ID from UUID format to short format.
-- Updates all FK references across apps and deploys.

UPDATE apps SET server_id = 'cp_ctrl_0000000001' WHERE server_id = '00000000-0000-0000-0000-000000000001';
UPDATE deploys SET server_id = 'cp_ctrl_0000000001' WHERE server_id = '00000000-0000-0000-0000-000000000001';
UPDATE server_metrics_history SET server_id = 'cp_ctrl_0000000001' WHERE server_id = '00000000-0000-0000-0000-000000000001';
UPDATE servers SET id = 'cp_ctrl_0000000001' WHERE id = '00000000-0000-0000-0000-000000000001';
