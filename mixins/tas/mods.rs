// Хук `native_mods`: регистрирует мод в списке Options -> Mods лаунчера/клиента Horizon
{
    mods.push(crate::game::native_hooks::NativeMod {
        id: "tas",
        title: "Horizon TAS Macro Recorder",
        description: "Запись и воспроизведение потиковых макросов (.htas) прямо в игре.",
    });
}
