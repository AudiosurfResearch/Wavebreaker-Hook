#define WIN32_LEAN_AND_MEAN
#include <Windows.h>
#include <WinInet.h>

#include <chrono>
#include <thread>
#include <stdexcept>

#include <spdlog/spdlog.h>
#include <spdlog/sinks/rotating_file_sink.h>

#include "config.hpp"
#include "nethook.hpp"
#include "overlay.hpp"

// Create a file rotating logger with 5 MB size max and 3 rotated files
auto max_size = 1048576 * 5;
auto max_log_files = 3;
auto logger = spdlog::rotating_logger_mt("wavebreaker_client", "logs/wavebreaker_log.txt", max_size, max_log_files);

uint32_t __stdcall init(void *args)
{
	spdlog::set_default_logger(logger);
	spdlog::set_level(spdlog::level::info);
	spdlog::flush_on(spdlog::level::debug);
	spdlog::info("Init");

	try
	{
		wavebreaker::config::init();
	}
	catch (const std::exception &e)
	{
		spdlog::error("Exception thrown when loading config: {}", e.what());
		MessageBoxA(nullptr, "Wavebreaker client config error!", "Error", MB_OK | MB_ICONERROR);
		return 1;
	}

	if (wavebreaker::config::verbose)
	{
		spdlog::set_level(spdlog::level::debug);
		spdlog::debug("Verbose logging enabled!");
	}

	while (!GetModuleHandleA("HTTP_Fetch_Unicode.dll") || !GetModuleHandleA("17C5B19F-4273-423C-A158-CA6F73046D43.dll"))
		Sleep(100);

	try
	{
		wavebreaker::nethook::init();
	}
	catch (const std::exception &e)
	{
		MessageBoxA(nullptr, "Wavebreaker hook failed!", "Error", MB_OK | MB_ICONERROR);
		return 1;
	}

	wavebreaker::overlay::init();

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
