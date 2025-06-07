# Changelog

## [0.12.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.11.2...v0.12.0) (2025-06-07)


### Features

* **cli:** `new-task` cli command ([#76](https://github.com/louis-thevenet/vault-tasks/issues/76)) ([5b44289](https://github.com/louis-thevenet/vault-tasks/commit/5b4428978def1d441c55668925e2f2e946a4effe))
* **cli:** makes new_task command accept multiple tasks and allow hyphen values without passing -- ([8b485cc](https://github.com/louis-thevenet/vault-tasks/commit/8b485cc3986eb4fa0f91c4a76ade0b7292739e24))
* shell completions generation ([#77](https://github.com/louis-thevenet/vault-tasks/issues/77)) ([9a69b5d](https://github.com/louis-thevenet/vault-tasks/commit/9a69b5dc3b633701f708050a626355a9adfddd31))
* **tui:** allow wrapping around when selecting previous or next tab ([ba105e6](https://github.com/louis-thevenet/vault-tasks/commit/ba105e6cd2225817a2bab04d91c19a36c414c97e))
* **tui:** display container name in preview ([#73](https://github.com/louis-thevenet/vault-tasks/issues/73)) ([c03d998](https://github.com/louis-thevenet/vault-tasks/commit/c03d998cb8637901bfb32c4f86bd08a51127d995))


### Bug Fixes

* **core:** strengthen time parsing rules (don't accept h&gt;24 etc) ([4841be2](https://github.com/louis-thevenet/vault-tasks/commit/4841be246cff467c8bfca857e1b2b143d31f944f))

## [0.12.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.11.2...v0.12.0) (2025-06-07)


### Features

* **cli:** `new-task` cli command ([#76](https://github.com/louis-thevenet/vault-tasks/issues/76)) ([5b44289](https://github.com/louis-thevenet/vault-tasks/commit/5b4428978def1d441c55668925e2f2e946a4effe))
* **cli:** makes new_task command accept multiple tasks and allow hyphen values without passing -- ([8b485cc](https://github.com/louis-thevenet/vault-tasks/commit/8b485cc3986eb4fa0f91c4a76ade0b7292739e24))
* shell completions generation ([#77](https://github.com/louis-thevenet/vault-tasks/issues/77)) ([9a69b5d](https://github.com/louis-thevenet/vault-tasks/commit/9a69b5dc3b633701f708050a626355a9adfddd31))
* **tui:** allow wrapping around when selecting previous or next tab ([ba105e6](https://github.com/louis-thevenet/vault-tasks/commit/ba105e6cd2225817a2bab04d91c19a36c414c97e))
* **tui:** display container name in preview ([#73](https://github.com/louis-thevenet/vault-tasks/issues/73)) ([c03d998](https://github.com/louis-thevenet/vault-tasks/commit/c03d998cb8637901bfb32c4f86bd08a51127d995))


### Bug Fixes

* **core:** strengthen time parsing rules (don't accept h&gt;24 etc) ([4841be2](https://github.com/louis-thevenet/vault-tasks/commit/4841be246cff467c8bfca857e1b2b143d31f944f))

## [0.11.2](https://github.com/louis-thevenet/vault-tasks/compare/v0.11.1...v0.11.2) (2025-05-30)


### Bug Fixes

* **calendar:** print previewed date instead of selected date ([f245e43](https://github.com/louis-thevenet/vault-tasks/commit/f245e4362af0c6449450d2172b11eb4e227df1bb))
* **config:** --config flag not always registering the provided file/path ([d231d46](https://github.com/louis-thevenet/vault-tasks/commit/d231d46ffb1d1083205386b466820be8144cabd8))
* **config:** pretty symbols did not fall back to default when deserializing config ([1a2dc32](https://github.com/louis-thevenet/vault-tasks/commit/1a2dc32cafd7f518120049c7915e772180f328db))
* **tasks:** improve relative dates generation ([b48af62](https://github.com/louis-thevenet/vault-tasks/commit/b48af62fcd4fb7a2d4f4ab5f0a1479202292e123))

## [0.11.1](https://github.com/louis-thevenet/vault-tasks/compare/v0.11.0...v0.11.1) (2025-05-18)


### Bug Fixes

* calendar view didn't display all day's tasks when time was also set ([f071cd0](https://github.com/louis-thevenet/vault-tasks/commit/f071cd0b3c9d4398c4115c3b0c495f254288bab4))

## [0.11.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.10.0...v0.11.0) (2025-04-13)


### Features

* **core:** don't parse tasks in comments and code blocks ([54c9a04](https://github.com/louis-thevenet/vault-tasks/commit/54c9a041e50491af34842fb0b3421cb9c4a01807))
* **time_management:** add config option to wait for input before skipping to new segment ([b9eef0b](https://github.com/louis-thevenet/vault-tasks/commit/b9eef0b504c8faeb46c830125b2a83c7b8111621))
* **tui:** add keybinds to increase/decrease completion from vault-tasks ([888e62e](https://github.com/louis-thevenet/vault-tasks/commit/888e62e75d2e12d7f9788c7e22baeba2721e4e49))


### Bug Fixes

* **core:** remove extra whitespace added to tasks with [@today](https://github.com/today) ([10e7869](https://github.com/louis-thevenet/vault-tasks/commit/10e78695b8ed835753d6ab4901c22d1a117b20d0))
* **tui:** wrong task widget height when only completion was set ([64fc87a](https://github.com/louis-thevenet/vault-tasks/commit/64fc87a87446beb94165ae3c5220012e3553062b))

## [0.10.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.9.0...v0.10.0) (2025-03-30)


### Features

* add a completion percentage to tasks displayed as a progression bar ([a8f916b](https://github.com/louis-thevenet/vault-tasks/commit/a8f916b43de07b33f2a0993f7c4d664fed619f37))

## [0.9.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.8.1...v0.9.0) (2025-01-29)


### Features

* wrapping for title and display nice markdown ([#39](https://github.com/louis-thevenet/vault-tasks/issues/39)) ([09a134e](https://github.com/louis-thevenet/vault-tasks/commit/09a134e767209eb8850caa9b8f130d6754622ddc))

## [0.8.1](https://github.com/louis-thevenet/vault-tasks/compare/v0.8.0...v0.8.1) (2025-01-10)


### Bug Fixes

* panic when vault was empty while trying to enter selected entry ([eb3b5de](https://github.com/louis-thevenet/vault-tasks/commit/eb3b5de009a4fb43e2f97b2e5467c34945f3048c))

## [0.8.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.7.0...v0.8.0) (2025-01-01)


### Features

* add calendar tab ([#37](https://github.com/louis-thevenet/vault-tasks/issues/37)) ([20120cd](https://github.com/louis-thevenet/vault-tasks/commit/20120cdcbb827f20a7380e8f24cf6bc9e237f6ed))


### Bug Fixes

* **calendar:** only preview selected day ([14292b4](https://github.com/louis-thevenet/vault-tasks/commit/14292b45d6431d9c1d630f0ee7977024923a4632))
* **tui:** changing state on wrong element type doesn't crash anymore but simply logs error ([9d8f4c8](https://github.com/louis-thevenet/vault-tasks/commit/9d8f4c85efefdcb9f3c58b313af2bf04f75e4261))

## [0.7.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.6.1...v0.7.0) (2024-12-21)


### Features

* add actions to mark tasks Done, ToDo, Incomplete and Canceled ([f07a13b](https://github.com/louis-thevenet/vault-tasks/commit/f07a13b9084f48f8520a1b81575b3f3976396c82))
* add Incomplete and Canceled states ([d98b378](https://github.com/louis-thevenet/vault-tasks/commit/d98b3784bf33149dea5357132787308b4175dfa4))
* **config:** add config options for task state markers ([ed9334f](https://github.com/louis-thevenet/vault-tasks/commit/ed9334fb2fa5bd498d4c4672b3c18d469fb6a16f))
* **config:** add config options for tui symbols (e.g. use ASCII sequences instead of emojis) ([734ae1a](https://github.com/louis-thevenet/vault-tasks/commit/734ae1a1298a07ce8f222ce4b26cda9ef1f9bbb2))

## [0.6.1](https://github.com/louis-thevenet/vault-tasks/compare/v0.6.0...v0.6.1) (2024-12-08)


### Bug Fixes

* **flowtime:** skipped break time is added to next break segment ([f8cbde3](https://github.com/louis-thevenet/vault-tasks/commit/f8cbde3f9de0714be04275909f4c34d1eb36b421))
* **pomodoro:** skipped break and focus time are added to next segment ([b944813](https://github.com/louis-thevenet/vault-tasks/commit/b944813aa077a1e26d1b2619fda1a6d8ed7690ff))

## [0.6.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.5.1...v0.6.0) (2024-11-30)


### Features

* add ToggleStatus action on tasks ([266e6b2](https://github.com/louis-thevenet/vault-tasks/commit/266e6b2de7f5f635150c6df307b4579217b3edb3))


### Bug Fixes

* crash when selected item was changed and did not match the active filter anymore ([3d7e948](https://github.com/louis-thevenet/vault-tasks/commit/3d7e948e4c22466323d6bd774b505bb7b69e1236))

## [0.5.1](https://github.com/louis-thevenet/vault-tasks/compare/v0.5.0...v0.5.1) (2024-11-24)


### Bug Fixes

* **config:** crash if no time management settings were provided in config file ([1f2f0a7](https://github.com/louis-thevenet/vault-tasks/commit/1f2f0a7748273aa5e2f7ba5e522cff81aa51dbac))
* **config:** merge highlight styles into Home section ([ada558e](https://github.com/louis-thevenet/vault-tasks/commit/ada558ed8c6b16ad919e027a9af8d05981d36a5d))

## [0.5.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.4.0...v0.5.0) (2024-11-24)


### Features

* add config option for time management methods ([70fbb59](https://github.com/louis-thevenet/vault-tasks/commit/70fbb594595bf9ece7fcd73b774b86c818155799))
* add Time Management tab ([#27](https://github.com/louis-thevenet/vault-tasks/issues/27)) ([8d5c12c](https://github.com/louis-thevenet/vault-tasks/commit/8d5c12ce8581f0ab9468a17bcf7ff7ea5801a987))
* **core:** include relative date in stdout mode ([7fdff9c](https://github.com/louis-thevenet/vault-tasks/commit/7fdff9c18087e8df4cbae5b1425ce451c423944b))


### Bug Fixes

* add a cli argument to show fps and tps counters ([3991042](https://github.com/louis-thevenet/vault-tasks/commit/3991042244f2e8bf7c8f46b2d70860ddfde38060))
* home component not registering when focusing time management tab ([6909a1c](https://github.com/louis-thevenet/vault-tasks/commit/6909a1c1b9769d833d4f9b01b960cc56fce70a28))
* time management settings wrong line selected on startup ([cf90476](https://github.com/louis-thevenet/vault-tasks/commit/cf90476853b39490f111fbc2152b0d11b7d74b2d))

## [0.4.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.3.0...v0.4.0) (2024-11-09)


### Features

* add config option for task state marker (default is 'x' to comply with prettier) ([f9341cd](https://github.com/louis-thevenet/vault-tasks/commit/f9341cd3fa17049d0542e1049c0f42d83c3e9103))
* **tui:** add config option to show time delta between due date and today's date ([f99a40f](https://github.com/louis-thevenet/vault-tasks/commit/f99a40fa02268f6ab01af45ea5753a0702374db2))
* **tui:** add help menu ([#20](https://github.com/louis-thevenet/vault-tasks/issues/20)) ([0cd64f4](https://github.com/louis-thevenet/vault-tasks/commit/0cd64f440555b5e7b60a8e93d050f99c1c8bcacf))
* **tui:** add sorting by due dates or titles to Filter tab ([#22](https://github.com/louis-thevenet/vault-tasks/issues/22)) ([43f2ca5](https://github.com/louis-thevenet/vault-tasks/commit/43f2ca567e76ab0c9d3fe2b54ac85c4ea3b1a9d1))
* **tui:** edit task from vault-task ([#21](https://github.com/louis-thevenet/vault-tasks/issues/21)) ([7df637a](https://github.com/louis-thevenet/vault-tasks/commit/7df637a025838fc04d24aa6b234a46cea553678c))
* **tui:** improve footers ([a0df9fc](https://github.com/louis-thevenet/vault-tasks/commit/a0df9fc81c5a372f8b2c79b140a6fca1cfc37cb1))
* **tui:** style relative date (dim) ([f99a40f](https://github.com/louis-thevenet/vault-tasks/commit/f99a40fa02268f6ab01af45ea5753a0702374db2))


### Bug Fixes

* **core/filter:** return too early when a task did not match the filter, preventing its children from being filtered ([a10dbdc](https://github.com/louis-thevenet/vault-tasks/commit/a10dbdcb6317bf5d05ec3691f65b00db67eaf227))
* **tui:** add hours when time delta &lt; 1 day in relative due date ([b8cbba3](https://github.com/louis-thevenet/vault-tasks/commit/b8cbba393c4d004c5e4dd5e4251aa6587ff6bef7))
* **tui:** today tag not taken into account in task widget height ([881f62e](https://github.com/louis-thevenet/vault-tasks/commit/881f62e953708988e5f8f3560e690598c293c6df))

## [0.3.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.2.0...v0.3.0) (2024-10-23)


### Features

* add search bar to vault explorer ([#16](https://github.com/louis-thevenet/vault-tasks/issues/16)) ([e2a66ba](https://github.com/louis-thevenet/vault-tasks/commit/e2a66ba4d1c1d73943f34175c8df16cd54de1a80))
* add special Today tag ([38751d2](https://github.com/louis-thevenet/vault-tasks/commit/38751d24fa8d9af00f6c96378764a9593d438f89))
* **cli:** add stdout command and improve stdout vault output ([c5f604a](https://github.com/louis-thevenet/vault-tasks/commit/c5f604ad073eefba7ca2936fe0015ee6321212ac))
* **cli:** open single markdown files ([cf73c40](https://github.com/louis-thevenet/vault-tasks/commit/cf73c4068723e69c9d3f8be26d70712f083daf8c))
* **config:** make default vault and config optional ([70231c8](https://github.com/louis-thevenet/vault-tasks/commit/70231c8dc54254a0071a5650aa0726ed738228ed))
* configurable color scheme ([541c91e](https://github.com/louis-thevenet/vault-tasks/commit/541c91ed28d20f70bbd53df43b587598d0a9e7cd))
* **core:** add config option to propagate file tags to all contained tasks ([762bf6c](https://github.com/louis-thevenet/vault-tasks/commit/762bf6c5fbc225e3fd86557e6dc40e3da2fd7a3f))
* **tui:** add reload action ([784ac47](https://github.com/louis-thevenet/vault-tasks/commit/784ac47ca57f0c86f01b4c20996ddfcee5de32ce))
* **tui:** add scrolling to lists ([37a672f](https://github.com/louis-thevenet/vault-tasks/commit/37a672fa78e1595cf66258cac24e7ac80379eb15))
* **tui:** display origin file in task widget ([#17](https://github.com/louis-thevenet/vault-tasks/issues/17)) ([72dd287](https://github.com/louis-thevenet/vault-tasks/commit/72dd28740f56b5b044b5e0c87f169739525b7311))
* **tui:** improve error reporting ([0d7ce2f](https://github.com/louis-thevenet/vault-tasks/commit/0d7ce2fff8b3e08a2efa0a5db823d3af4c1434cb))
* **tui:** open selected entry in default editor ([#18](https://github.com/louis-thevenet/vault-tasks/issues/18)) ([4cc4177](https://github.com/louis-thevenet/vault-tasks/commit/4cc417721c0356ded734bd7c2fdb1526969a12d7))
* use TOML as default format for config ([3e25a6c](https://github.com/louis-thevenet/vault-tasks/commit/3e25a6ced8302ac460406072e2be8f9c6d694373))


### Bug Fixes

* **cli:** make vault_path a named argument (prevents from running commands) ([691aaa7](https://github.com/louis-thevenet/vault-tasks/commit/691aaa7913e1b097ff6edcc27718efdc9d460112))
* **core:** Bad parent for nested tasks ([db92820](https://github.com/louis-thevenet/vault-tasks/commit/db928207acd9ddd9f97116ec75ca7130c2509311))
* **core:** empty directories being added to vault ([5d49b40](https://github.com/louis-thevenet/vault-tasks/commit/5d49b40f05f6b4fbe0e00b26eff3df04b7d662cb))
* **core:** nested task incorrectly displayed in Display implementation of VaultData ([807e7ff](https://github.com/louis-thevenet/vault-tasks/commit/807e7ff1b5500eb817fa9724b1191d78b745531e))
* **core:** task indenting no longer gets deleted ([49e2402](https://github.com/louis-thevenet/vault-tasks/commit/49e24025dbd253c2e377531f36320d2a74cf5d68))
* **explorer:** make file name bold in path instead of showing in preview ([cb693d0](https://github.com/louis-thevenet/vault-tasks/commit/cb693d07d70821cd1fa9d762f6bb2fe611316784))
* **filter_tab:** tags list filter no longer case sensitive ([a4f308a](https://github.com/louis-thevenet/vault-tasks/commit/a4f308a420bb81e1cd89460b3521b56e7f2e8250))
* **parser:** make parser less strict on extra indented descriptions by adding them to closest parent ([86b30b4](https://github.com/louis-thevenet/vault-tasks/commit/86b30b40dc47bda9fbb5f801f6be3f1194a4d531))
* **tui:** wrong height for subtasks ([fc1374d](https://github.com/louis-thevenet/vault-tasks/commit/fc1374d4314090a2c0d81c2391b1d488c642ff4a))

## [0.2.0](https://github.com/louis-thevenet/vault-tasks/compare/v0.1.0...v0.2.0) (2024-09-29)


### Features

* **tui:** add filter tab ([#13](https://github.com/louis-thevenet/vault-tasks/issues/13)) ([825523a](https://github.com/louis-thevenet/vault-tasks/commit/825523ada89134c3637af3127af1c9c8e9cd1b7c))
* **tui:** add tab bar ([f1344f0](https://github.com/louis-thevenet/vault-tasks/commit/f1344f0c95a332826074925071994dac7e718d28))
* **tui:** sort explorer entries ([#12](https://github.com/louis-thevenet/vault-tasks/issues/12)) ([ee38072](https://github.com/louis-thevenet/vault-tasks/commit/ee38072e8596f3948f42b7445943bcbad80e65fa))
* **vault_parser:** only parse markdown files ([cfb8db1](https://github.com/louis-thevenet/vault-tasks/commit/cfb8db1e6e7c493d96349c44895b7a6fba4f0d81))


### Bug Fixes

* **core:** remove extra whitespace when fixing tasks in files ([d8a2a86](https://github.com/louis-thevenet/vault-tasks/commit/d8a2a86975710eb20cfd202237a5af77f01c6b7c))
* **explorer:** only sort file and directory entries ([c2abfef](https://github.com/louis-thevenet/vault-tasks/commit/c2abfef535fbc978089f925c8c61cb14f340bbfc))
* **tui:** false error on startup : initiate entries list on creation ([0a9e7ed](https://github.com/louis-thevenet/vault-tasks/commit/0a9e7ed19b50a1d3bcc5001b682cd49e4a9c25fa))

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
* make default task name an empty string ([77c1528](https://github.com/louis-thevenet/vault-tasks/commit/77c152879dd7994d1ef8398cf82f53a16dd47927))
* replaced "next monday", "next week" patterns with "2weeks" "1month" ([8927696](https://github.com/louis-thevenet/vault-tasks/commit/8927696802952ea40f8f62feb3521c992246eb74))
* to_lowercase() has no effect since it can only matched lowercased words ([fa488b0](https://github.com/louis-thevenet/vault-tasks/commit/fa488b0e05d422dfe047f409aa49229b54a6bb3c))
* use default task name when no name is provided ([529be0f](https://github.com/louis-thevenet/vault-tasks/commit/529be0fd20137c3f4055f0e62f67b4faf9695958))
