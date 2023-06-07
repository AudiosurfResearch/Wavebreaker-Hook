#include "framework.h"
#include <chrono>
#include <thread>
#include <stdexcept>
#include "SafetyHook/safetyhook.hpp"
#include "ini.h" //mINI

SafetyHookInline g_getserverunicode_hook{};
SafetyHookInline g_getserver_hook{};
SafetyHookInline g_internetconnecta_hook{};
SafetyHookInline g_httpopenrequesta_hook{};


mINI::INIFile file("Wavebreaker-Hook.ini");
mINI::INIStructure ini;
std::string newServer;

char* rewriteTargetServer(char* server) {
	if (strstr(server, "audio-surf") || strstr(server, "audiosurfthegame")) server = _strdup(newServer.c_str());
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

HINTERNET WINAPI InternetConnectHook(HINTERNET hInternet,
	LPCSTR        lpszServerName,
	INTERNET_PORT nServerPort,
	LPCSTR        lpszUserName,
	LPCSTR        lpszPassword,
	DWORD         dwService,
	DWORD         dwFlags,
	DWORD_PTR     dwContext)
{
	if (nServerPort == 80)
		nServerPort = 443;
	return g_internetconnecta_hook.stdcall<HINTERNET>(hInternet, lpszServerName, nServerPort, lpszUserName, lpszPassword, dwService, dwFlags, dwContext);
}

HINTERNET WINAPI OpenRequestHook(HINTERNET hConnect,
	LPCSTR    lpszVerb,
	LPCSTR    lpszObjectName,
	LPCSTR    lpszVersion,
	LPCSTR    lpszReferrer,
	LPCSTR* lplpszAcceptTypes,
	DWORD     dwFlags,
	DWORD_PTR dwContext)
{
	if (!dwFlags)
		dwFlags = INTERNET_FLAG_SECURE;
	if (dwFlags == INTERNET_FLAG_RELOAD)
		dwFlags = INTERNET_FLAG_RELOAD | INTERNET_FLAG_SECURE;

	return g_httpopenrequesta_hook.stdcall<HINTERNET>(hConnect,
		lpszVerb,
		lpszObjectName,
		lpszVersion,
		lpszReferrer,
		lplpszAcceptTypes,
		dwFlags,
		dwContext);
}

uint32_t __stdcall init(void* args) {
	while (!GetModuleHandleA("HTTP_Fetch_Unicode.dll") || !GetModuleHandleA("17C5B19F-4273-423C-A158-CA6F73046D43.dll")) Sleep(100);

	file.read(ini);
	if (ini["Config"].has("server")) newServer = ini.get("Config").get("server");
	else MessageBoxA(nullptr, "Wavebreaker hook config error!", "Error", MB_OK | MB_ICONERROR);

	FARPROC targetServerUnicodeHandle = GetProcAddress(GetModuleHandleA("HTTP_Fetch_Unicode.dll"), "?GetTargetServer@HTTP_Fetch_Unicode@@UAEPADXZ");
	FARPROC targetServerHandle = GetProcAddress(GetModuleHandleA("17C5B19F-4273-423C-A158-CA6F73046D43.dll"), "?GetTargetServer@Aco_HTTP_Fetch@@UAEPADXZ");
	g_getserverunicode_hook = safetyhook::create_inline((uintptr_t)targetServerUnicodeHandle, (uintptr_t)GetTargetServerUnicodeHook);
	g_getserver_hook = safetyhook::create_inline((uintptr_t)targetServerHandle, (uintptr_t)GetTargetServerHook);
	g_internetconnecta_hook = safetyhook::create_inline((uintptr_t)InternetConnectA, (uintptr_t)InternetConnectHook);
	g_httpopenrequesta_hook = safetyhook::create_inline((uintptr_t)HttpOpenRequestA, (uintptr_t)OpenRequestHook);
	if (!g_getserver_hook || !g_getserverunicode_hook || !g_internetconnecta_hook || !g_httpopenrequesta_hook)
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
