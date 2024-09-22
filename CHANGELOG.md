# Changelog

## 0.1.0 (2024-09-22)


### Features

* feat:  ([b05e215](https://github.com/louis-thevenet/vault-tasks/commit/b05e215db206bd96cc68d42f4a4be8a2d9db515e))
* add better logging when indenting is wrong in subtasks and task descriptions ([7ddca06](https://github.com/louis-thevenet/vault-tasks/commit/7ddca0689e57c4c5f8d04e421537b4cbfe900e0a))
* add config loading + basic vault scanner ([0a44408](https://github.com/louis-thevenet/vault-tasks/commit/0a444086a13f78638bb5b76b8c0e4ad4f5e08535))
* add config option for american date format ([3895b33](https://github.com/louis-thevenet/vault-tasks/commit/3895b33815425bf61bc229b1c085a6693fd1a6eb))
* add due date, priority, state and tags to tasks ([ecb3fcf](https://github.com/louis-thevenet/vault-tasks/commit/ecb3fcf9081eae3bae75a7d9555bd37d8a966666))
* add generate-config command ([1b0e74f](https://github.com/louis-thevenet/vault-tasks/commit/1b0e74fe254c43942ac2e3fdef8ef8f96fb2a942))
* add ignored_paths config option ([6c725f2](https://github.com/louis-thevenet/vault-tasks/commit/6c725f216c3e1cbc2816e560c4bcb4bcc13705bf))
* add parser for Task ([6f5592e](https://github.com/louis-thevenet/vault-tasks/commit/6f5592ee71f77ddd94dd4621eda8f45c83c27f2f))
* add support for subtasks, and cleaning of useless jeaders ([a8dae9f](https://github.com/louis-thevenet/vault-tasks/commit/a8dae9fdb28d2cc5088f5cad836fdc5268f278cd))
* add support for task descriptions ([e34b52b](https://github.com/louis-thevenet/vault-tasks/commit/e34b52bedbd24f7b545a7a3595e4dcde140d6928))
* all relative dates (2weeks, monday, etc) are replaced by hard numeric dates after parsing ([d134011](https://github.com/louis-thevenet/vault-tasks/commit/d1340112f65ceb8bd3259e0667cc337991e82bd2))
* better errors in home component ([69ab53e](https://github.com/louis-thevenet/vault-tasks/commit/69ab53e4897402efb0b9e97a7fe97754460801ea))
* better navigation with previous entry, current entry content and preview windows ([1ca8052](https://github.com/louis-thevenet/vault-tasks/commit/1ca8052dac7038e02592e5aa699abe31e68c489c))
* display entries name in lateral menu + made it easy to later add selection ([7b57425](https://github.com/louis-thevenet/vault-tasks/commit/7b574253de8732f9029e453a9021610de4c999e1))
* fail when no config is found and create default config ([c95b510](https://github.com/louis-thevenet/vault-tasks/commit/c95b510720092ce37761b4f3bb3e51104eb970fd))
* file can be added to ignored_paths as well ([75bc4f8](https://github.com/louis-thevenet/vault-tasks/commit/75bc4f8b441942fb3fe00a9c769b802082342165))
* improve FileEntry's to_string ([ca213da](https://github.com/louis-thevenet/vault-tasks/commit/ca213da02e0eee3e31528d70939bc3548b8798bc))
* improve VaultData structure to make Directory, Header & SubTasks distinct ([7b57425](https://github.com/louis-thevenet/vault-tasks/commit/7b574253de8732f9029e453a9021610de4c999e1))
* invert ignore_dot_files config option ([25639ed](https://github.com/louis-thevenet/vault-tasks/commit/25639ed5547813bdda356f469e9ff307c2908520))
* move Task to task.rs + make clippy happy ([b70fe65](https://github.com/louis-thevenet/vault-tasks/commit/b70fe650f30b15baa86dc372d6956e5373ea88de))
* only keep relative path for tasks ([c65f6b2](https://github.com/louis-thevenet/vault-tasks/commit/c65f6b210db04a7df561e6562f4ddd677061a2d4))
* parse tasks from markdown vault ([75f58e9](https://github.com/louis-thevenet/vault-tasks/commit/75f58e9936e4a71e5888e3cc490d77a03d389814))
* Parser prototype is working and able to parse tasks and headers from vault ([e9331f4](https://github.com/louis-thevenet/vault-tasks/commit/e9331f41862bae83e87cc0510b77342c54b271a8))
* preview selected task instead of its children ([76b9460](https://github.com/louis-thevenet/vault-tasks/commit/76b9460bdebf3a24c456d91c95b553e81b54a3ca))
* scanner reads tasks ([ac4ecae](https://github.com/louis-thevenet/vault-tasks/commit/ac4ecae6412668d7b6553db3bc0b4fb88fd8c1ee))
* separate entries names into prefixes and actual path name ([57badaf](https://github.com/louis-thevenet/vault-tasks/commit/57badaf36fd95a726ca59b2808314ecf5d14c83d))
* setup rust ([51b0b5a](https://github.com/louis-thevenet/vault-tasks/commit/51b0b5a62f1999edc1aba2400b5487ba03e77522))
* **Task:** store line number for future editing, improve time related data ([385366b](https://github.com/louis-thevenet/vault-tasks/commit/385366b479dbaba401f54ce789fe5dc72628494f))
* use widget list instead of text list for better styling ([26b7cd8](https://github.com/louis-thevenet/vault-tasks/commit/26b7cd849c6df7f61bfe5e7f868cb7c1b67ef712))
* working navigation through vault data ([a8a8cc5](https://github.com/louis-thevenet/vault-tasks/commit/a8a8cc530622a6ea301871b87b5f7ea3bb9116bc))


### Bug Fixes

* don't create config file if running tests ([c65c7be](https://github.com/louis-thevenet/vault-tasks/commit/c65c7beb7d3ba6878dc464ce324384f562632fe1))
* don't use Option types in Config ([70a0244](https://github.com/louis-thevenet/vault-tasks/commit/70a0244f333f404cb369663da5ba5bfbab31b662))
* fix some warnings ([1873759](https://github.com/louis-thevenet/vault-tasks/commit/1873759f8ea954167038dacb3a43781293b593e2))
* replaced "next monday", "next week" patterns with "2weeks" "1month" ([8927696](https://github.com/louis-thevenet/vault-tasks/commit/8927696802952ea40f8f62feb3521c992246eb74))
* to_lowercase() has no effect since it can only matched lowercased words ([fa488b0](https://github.com/louis-thevenet/vault-tasks/commit/fa488b0e05d422dfe047f409aa49229b54a6bb3c))
* use default task name when no name is provided ([529be0f](https://github.com/louis-thevenet/vault-tasks/commit/529be0fd20137c3f4055f0e62f67b4faf9695958))
