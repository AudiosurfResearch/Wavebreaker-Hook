#include <Windows.h>

namespace wavebreaker
{
    namespace utility
    {
        bool is_key_down(int key)
        {
            return (GetAsyncKeyState(key) & (1 << 15)) != 0;
        }

        bool was_key_pressed(int key)
        {
            static bool keys[0xFF]{false};

            if (is_key_down(key) && !keys[key])
            {
                keys[key] = true;

                return true;
            }

            if (!is_key_down(key))
            {
                keys[key] = false;
            }

            return false;
        }
    }
}