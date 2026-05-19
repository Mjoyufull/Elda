-- Optional post-install hook.
--
-- Lifecycle hooks are exceptional: declarative metadata in pkg.lua should
-- handle conffiles, sysusers, tmpfiles, alternatives, and provider_assets
-- without dropping into a hook. Use a hook only when the work is genuinely
-- imperative.
--
-- The hook runs in Elda's embedded Lua sandbox after staging completes and
-- before activation publishes the new state.

elda.log.info("example-daemon: refreshing operator-friendly defaults")

-- Touch the conffile so a fresh install gets a non-empty file even if the
-- packaged copy is empty for any reason. Conffile handling itself is owned by
-- the conffile system declared in pkg.lua, not by this hook.
local conf = "/etc/example-daemon/example-daemon.conf"
if not elda.fs.exists(conf) then
  elda.fs.write_text(conf, "log_level = info\n")
end
