module.exports = {
  extends: ['@commitlint/config-conventional'],
  ignores: [
    (commit) => commit.startsWith('chore(deps)'),
    (commit) => commit.startsWith('WIP'),
  ],
};
