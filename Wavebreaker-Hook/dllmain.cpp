#include "framework.h"
#include <chrono>
#include <thread>
#include <stdexcept>
#include "SafetyHook/safetyhook.hpp"

SafetyHookInline g_getserver_hook{};

char* __fastcall GetTargetServerHook(void* thisptr, uintptr_t edx)
{
	char* server = g_getserver_hook.thiscall<char*>(thisptr);
	MessageBoxA(nullptr, server, "yea", MB_OK | MB_ICONERROR);
	return server;
	
}

unsigned long WINAPI initialize(void* instance) {
	while (!GetModuleHandleA("HTTP_Fetch_Unicode.dll"))
		Sleep(200);

	try {
		FARPROC getTargetServerHandle = GetProcAddress(GetModuleHandleA("HTTP_Fetch_Unicode.dll"), "?GetTargetServer@HTTP_Fetch_Unicode@@UAEPADXZ");
		g_getserver_hook = safetyhook::create_inline((void*)getTargetServerHandle, (void*)GetTargetServerHook);
		if (!g_getserver_hook)
		{
			throw "Hook failed";
		}
	}
	catch (const std::runtime_error& error) {
		MessageBoxA(nullptr, error.what(), "Wavebreaker hook error!", MB_OK | MB_ICONERROR);
		FreeLibraryAndExitThread(static_cast<HMODULE>(instance), 0);
	}

	while (true)
		std::this_thread::sleep_for(std::chrono::milliseconds(50));

	FreeLibraryAndExitThread(static_cast<HMODULE>(instance), 0);
}


BOOL WINAPI DllMain(HMODULE handle, DWORD reason, LPVOID reserved)
{
	if (reason == DLL_PROCESS_ATTACH)
	{
		DisableThreadLibraryCalls(handle);

		if (const auto thread = CreateThread(nullptr, NULL, initialize, handle, NULL, nullptr))
		{
			CloseHandle(thread);
			return 1;
		}
	}

	return 0;
}

