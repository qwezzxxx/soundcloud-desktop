import struct
import ctypes
from ctypes import wintypes
import os

kernel32 = ctypes.windll.kernel32
shell32 = ctypes.windll.shell32

kernel32.BeginUpdateResourceW.argtypes = [wintypes.LPCWSTR, wintypes.BOOL]
kernel32.BeginUpdateResourceW.restype = wintypes.HANDLE

kernel32.UpdateResourceW.argtypes = [
    wintypes.HANDLE,
    ctypes.c_void_p,
    ctypes.c_void_p,
    wintypes.WORD,
    ctypes.c_void_p,
    wintypes.DWORD
]
kernel32.UpdateResourceW.restype = wintypes.BOOL

kernel32.EndUpdateResourceW.argtypes = [wintypes.HANDLE, wintypes.BOOL]
kernel32.EndUpdateResourceW.restype = wintypes.BOOL

RT_ICON = 3
RT_GROUP_ICON = 14

def embed_icon(exe_path, ico_path):
    if not os.path.exists(exe_path) or not os.path.exists(ico_path):
        print(f"File not found: {exe_path} or {ico_path}")
        return False

    with open(ico_path, "rb") as f:
        ico_data = f.read()

    idReserved, idType, idCount = struct.unpack("<HHH", ico_data[:6])
    
    entries = []
    for i in range(idCount):
        entry_offset = 6 + i * 16
        entry = ico_data[entry_offset:entry_offset+16]
        w, h, colors, res, planes, bpp, size, offset = struct.unpack("<BBBBHHII", entry)
        entries.append({
            "id": i + 1,
            "w": w,
            "h": h,
            "colors": colors,
            "planes": planes,
            "bpp": bpp,
            "size": size,
            "data": ico_data[offset:offset+size]
        })

    grp_header = struct.pack("<HHH", 0, 1, idCount)
    grp_entries = b""
    for e in entries:
        grp_entries += struct.pack("<BBBBHHIH", e["w"], e["h"], e["colors"], 0, e["planes"], e["bpp"], e["size"], e["id"])
    grp_data = grp_header + grp_entries

    hUpdate = kernel32.BeginUpdateResourceW(exe_path, False)
    if not hUpdate:
        print(f"BeginUpdateResourceW failed: {kernel32.GetLastError()}")
        return False

    for e in entries:
        raw_bytes = (ctypes.c_ubyte * len(e["data"])).from_buffer_copy(e["data"])
        kernel32.UpdateResourceW(hUpdate, RT_ICON, e["id"], 0, ctypes.byref(raw_bytes), len(e["data"]))

    grp_bytes = (ctypes.c_ubyte * len(grp_data)).from_buffer_copy(grp_data)
    for gid in [1, 32512]:
        kernel32.UpdateResourceW(hUpdate, RT_GROUP_ICON, gid, 0, ctypes.byref(grp_bytes), len(grp_data))

    ok = kernel32.EndUpdateResourceW(hUpdate, False)
    if not ok:
        print(f"EndUpdateResourceW failed: {kernel32.GetLastError()}")
        return False

    print(f"Icons embedded into {exe_path}")
    
    # Notify shell of icon update
    SHCNE_ASSOCCHANGED = 0x08000000
    SHCNE_UPDATEITEM = 0x00002000
    SHCNF_IDLIST = 0x0000
    SHCNF_PATHW = 0x0005
    shell32.SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, ctypes.c_wchar_p(exe_path), None)
    shell32.SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None)
    return True

if __name__ == "__main__":
    base_dir = os.path.dirname(os.path.abspath(__file__))
    ico = os.path.join(base_dir, "src-tauri", "icons", "icon.ico")
    p1 = os.path.join(base_dir, "SoundCloud.exe")
    p2 = os.path.join(base_dir, "src-tauri", "target", "debug", "soundcloud-desktop.exe")
    embed_icon(p1, ico)
    embed_icon(p2, ico)
