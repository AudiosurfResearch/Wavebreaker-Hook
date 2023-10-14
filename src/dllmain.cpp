#define WIN32_LEAN_AND_MEAN
#include <Windows.h>
#include <WinInet.h>

#include <chrono>
#include <thread>
#include <stdexcept>

#include <safetyhook.hpp>
#include <toml.hpp>
#include <spdlog/spdlog.h>
#include <spdlog/sinks/rotating_file_sink.h>

SafetyHookInline g_getserverunicode_hook{};
SafetyHookInline g_getserver_hook{};
SafetyHookInline g_internetconnecta_hook{};
SafetyHookInline g_httpopenrequesta_hook{};

auto data = toml::parse("Wavebreaker-Client.toml");
std::string newServer;
bool verboseLog;

// Create a file rotating logger with 5 MB size max and 3 rotated files
auto max_size = 1048576 * 5;
auto max_log_files = 3;
auto logger = spdlog::rotating_logger_mt("wavebreaker_client", "logs/wavebreaker_log.txt", max_size, max_log_files);

char *rewriteTargetServer(char *server)
{
	if (strstr(server, "audio-surf") || strstr(server, "audiosurfthegame"))
		server = _strdup(newServer.c_str());
	return server;
}

char *__fastcall GetTargetServerUnicodeHook(void *thisptr, uintptr_t edx)
{
	char *server = g_getserverunicode_hook.thiscall<char *>(thisptr);
	spdlog::debug("Rewriting server (Unicode): {0}", server);
	return rewriteTargetServer(server);
}

char *__fastcall GetTargetServerHook(void *thisptr, uintptr_t edx)
{
	char *server = g_getserver_hook.thiscall<char *>(thisptr);
	spdlog::debug("Rewriting server: {0}", server);
	return rewriteTargetServer(server);
}

HINTERNET WINAPI InternetConnectHook(HINTERNET hInternet,
									 LPCSTR lpszServerName,
									 INTERNET_PORT nServerPort,
									 LPCSTR lpszUserName,
									 LPCSTR lpszPassword,
									 DWORD dwService,
									 DWORD dwFlags,
									 DWORD_PTR dwContext)
{
	spdlog::debug("InternetConnect hook hit: {0} {1}", lpszServerName, nServerPort);
	if (nServerPort == 80)
		nServerPort = 443;
	return g_internetconnecta_hook.stdcall<HINTERNET>(hInternet, lpszServerName, nServerPort, lpszUserName, lpszPassword, dwService, dwFlags, dwContext);
}

HINTERNET WINAPI OpenRequestHook(HINTERNET hConnect,
								 LPCSTR lpszVerb,
								 LPCSTR lpszObjectName,
								 LPCSTR lpszVersion,
								 LPCSTR lpszReferrer,
								 LPCSTR *lplpszAcceptTypes,
								 DWORD dwFlags,
								 DWORD_PTR dwContext)
{
	if (!dwFlags)
		dwFlags = INTERNET_FLAG_SECURE;
	if (dwFlags == INTERNET_FLAG_RELOAD)
		dwFlags = INTERNET_FLAG_RELOAD | INTERNET_FLAG_SECURE;

	spdlog::debug("OpenRequest hook hit: {0} {1} {2} {3}", lpszVersion, lpszVerb, lpszReferrer, lpszObjectName);
	return g_httpopenrequesta_hook.stdcall<HINTERNET>(hConnect,
													  lpszVerb,
													  lpszObjectName,
													  lpszVersion,
													  lpszReferrer,
													  lplpszAcceptTypes,
													  dwFlags,
													  dwContext);
}

uint32_t __stdcall init(void *args)
{
	spdlog::set_default_logger(logger);
	spdlog::set_level(spdlog::level::info);
	spdlog::flush_on(spdlog::level::debug);
	spdlog::info("Init");

	spdlog::info("Loading config");
	try
	{
		newServer = toml::find<std::string>(data, "server");
		verboseLog = toml::find<bool>(data, "verbose");
	}
	catch (const std::exception &e)
	{
		spdlog::error("Exception thrown when loading config: {}", e.what());
		MessageBoxA(nullptr, "Wavebreaker client config error!", "Error", MB_OK | MB_ICONERROR);
		return 1;
	}

	if (verboseLog)
	{
		spdlog::set_level(spdlog::level::debug);
		spdlog::debug("Verbose logging enabled!");
	}

	while (!GetModuleHandleA("HTTP_Fetch_Unicode.dll") || !GetModuleHandleA("17C5B19F-4273-423C-A158-CA6F73046D43.dll"))
		Sleep(100);

	spdlog::info("Attaching hooks");
	FARPROC targetServerUnicodeHandle = GetProcAddress(GetModuleHandleA("HTTP_Fetch_Unicode.dll"), "?GetTargetServer@HTTP_Fetch_Unicode@@UAEPADXZ");
	FARPROC targetServerHandle = GetProcAddress(GetModuleHandleA("17C5B19F-4273-423C-A158-CA6F73046D43.dll"), "?GetTargetServer@Aco_HTTP_Fetch@@UAEPADXZ");
	g_getserverunicode_hook = safetyhook::create_inline((void *)targetServerUnicodeHandle, (void *)GetTargetServerUnicodeHook);
	g_getserver_hook = safetyhook::create_inline((void *)targetServerHandle, (void *)GetTargetServerHook);
	g_internetconnecta_hook = safetyhook::create_inline((void *)InternetConnectA, (void *)InternetConnectHook);
	g_httpopenrequesta_hook = safetyhook::create_inline((void *)HttpOpenRequestA, (void *)OpenRequestHook);
	if (!g_getserver_hook || !g_getserverunicode_hook || !g_internetconnecta_hook || !g_httpopenrequesta_hook)
	{
		spdlog::error("Failed to attach hook(s). Hook addresses: {0:p} {1:p} {2:p} {3:p}", fmt::ptr(&g_getserver_hook), fmt::ptr(&g_getserverunicode_hook), fmt::ptr(&g_internetconnecta_hook), fmt::ptr(&g_httpopenrequesta_hook));
		MessageBoxA(nullptr, "Wavebreaker hook failed!", "Error", MB_OK | MB_ICONERROR);
		return 1;
	}
	spdlog::info("Done");

	while (true)
		std::this_thread::sleep_for(std::chrono::milliseconds(50));

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
