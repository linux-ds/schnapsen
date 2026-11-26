# Contributing

Schnapsen is in active development. Check the [roadmap](PLAN.md), [specification](SPEC.md), and [rule audit](RULE_AUDIT.md) before proposing changes. For a bug report, include the app version, rule profile, and steps to reproduce it. Use an isolated test case when sharing game data.

- Keep changes focused. Keep game rules in the engine and user-facing strings in the gettext catalogs.
- Use GNOME Builder and the GNOME SDK 51 as described in [README.md](README.md). Run formatting and the tests relevant to your change. For rule changes, add a reproducible case and document the rule decision.
- Review the diff before committing. Keep credentials, personal paths, save files, and build artifacts out of commits.

See [the license assignments](README.md#licensing) before contributing. Identify any third-party material, include its source and license, and preserve its notices.
