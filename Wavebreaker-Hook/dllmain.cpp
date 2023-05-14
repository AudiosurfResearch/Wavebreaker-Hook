#include "framework.h"
#include <chrono>
#include <thread>
#include <stdexcept>
#include "SafetyHook/safetyhook.hpp"

SafetyHookInline g_getserverunicode_hook{};
SafetyHookInline g_getserver_hook{};

char* rewriteTargetServer(char* server) {
	if (strstr(server, "audio-surf") || strstr(server, "audiosurfthegame")) server = _strdup("127.0.0.1"); //TODO: Add config file?
	return server;
}

char* __fastcall GetTargetServerUnicodeHook(void* thisptr, uintptr_t edx)
{
	char* server = g_getserverunicode_hook.thiscall<char*>(thisptr);
	return rewriteTargetServer(server);
}

char* __fastcall GetTargetServerHook(void* thisptr, uintptr_t edx)
{
	char* server = g_getserver_hook.thiscall<char*>(thisptr);
	return rewriteTargetServer(server);
}

uint32_t __stdcall init(void* args) {
	while (!GetModuleHandleA("HTTP_Fetch_Unicode.dll") || !GetModuleHandleA("17C5B19F-4273-423C-A158-CA6F73046D43.dll")) Sleep(100);

	FARPROC targetServerUnicodeHandle = GetProcAddress(GetModuleHandleA("HTTP_Fetch_Unicode.dll"), "?GetTargetServer@HTTP_Fetch_Unicode@@UAEPADXZ");
	FARPROC targetServerHandle = GetProcAddress(GetModuleHandleA("17C5B19F-4273-423C-A158-CA6F73046D43.dll"), "?GetTargetServer@Aco_HTTP_Fetch@@UAEPADXZ");
	g_getserverunicode_hook = safetyhook::create_inline((uintptr_t)targetServerUnicodeHandle, (uintptr_t)GetTargetServerUnicodeHook);
	g_getserver_hook = safetyhook::create_inline((uintptr_t)targetServerHandle, (uintptr_t)GetTargetServerHook);
	if (!g_getserver_hook || !g_getserverunicode_hook)
	{
		MessageBoxA(nullptr, "Wavebreaker hook failed!", "Error", MB_OK | MB_ICONERROR);
		return 1;
	}

	while (true) std::this_thread::sleep_for(std::chrono::milliseconds(50));

	return 0;
}


BOOL WINAPI DllMain(HMODULE handle, DWORD reason, LPVOID reserved)
{
	if (reason == DLL_PROCESS_ATTACH)
	{
		DisableThreadLibraryCalls(handle);

		if (const auto thread = (HANDLE)_beginthreadex(nullptr, 0, &init, nullptr, 0, nullptr))
		{
			CloseHandle(thread);
			return 1;
		}
	}

	return 0;
}
