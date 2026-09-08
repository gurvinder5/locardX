We are developing LocardX, a forensic recovery and secure data sanitization
desktop application.

The repository is a two-developer GitHub project.

Read the existing:
- Problemstatement.txt
- Tech-Stack.txt
- UserRequirements.txt
- Secure Drive Eraser architecture documentation

Do not implement application functionality yet.

Create CONTRIBUTING.md containing our development rules.

The rules must include:

1. main is protected and must never receive direct feature commits.
2. develop is the integration branch.
3. All feature work must use feature/<name> branches.
4. Never force push.
5. Never use git reset --hard to discard work.
6. Never modify another developer's feature branch.
7. Keep commits small and logically grouped.
8. Every feature must include tests where applicable.
9. Run formatting, linting and tests before creating a PR.
10. Do not modify unrelated files.
11. Do not introduce dependencies without justification.
12. Do not silently change public interfaces.
13. Shared interfaces/types must be agreed upon before implementation.
14. Never commit secrets, credentials, private keys, disk images, recovered
    evidence, or sensitive forensic data.
15. Destructive disk/file operations must have explicit safety controls and
    tests using mock/test devices rather than real user disks.
16. Do not use force push or destructive Git commands.
17. Do not rewrite another developer's commits.

Also define the expected commit message format and PR requirements.

Do not create unnecessary files. 