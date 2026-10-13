/*
 * hello-rs C shim: hardware init/teardown + integer-only draw surface.
 *
 * Contract (see emotion-rs/examples/hello-rs/src/ffi.rs): every hr_
 * function takes/returns only 32-bit integers by value. ee-gcc is
 * hard-float o32; the Rust target is +soft-float, so floats must
 * never cross this boundary. Colors cross as the packed u32 RGBAQ
 * low word (== GS_SETREG_RGBAQ(r,g,b,a,0) widened).
 */
#include <gsKit.h>
#include <dmaKit.h>
#include <libpad.h>
#include <tamtypes.h>
#include <kernel.h>
#include <sifrpc.h>
#include <loadfile.h>

int rust_main(u32 width, u32 height);

static GSGLOBAL *gs;
static char padBuf[256] __attribute__((aligned(64)));
static u32 spriteRgba; /* color for hr_sprite, set via hr_set_color */

/* z-layers match the C demo: 1 gradient, 2/3 gem, 4 sprite */
#define Z_GRADIENT 1
#define Z_GEM_A    2
#define Z_GEM_B    3
#define Z_SPRITE   4

u32 hr_pad_buttons(void)
{
	struct padButtonStatus btn;

	if (padRead(0, 0, &btn) == 0)
		return 0;
	return (u32)(0xffff ^ btn.btns);
}

void hr_clear(u32 rgba)
{
	gsKit_clear(gs, (u64)rgba);
}

void hr_gradient(void)
{
	gsKit_prim_quad_gouraud(gs, 0, 0, 0, (float)gs->Height,
			(float)gs->Width, 0, (float)gs->Width, (float)gs->Height, Z_GRADIENT,
			GS_SETREG_RGBAQ(0x30,0x00,0x60,0x00,0x00),
			GS_SETREG_RGBAQ(0x00,0x00,0x00,0x00,0x00),
			GS_SETREG_RGBAQ(0x00,0x30,0x50,0x00,0x00),
			GS_SETREG_RGBAQ(0x00,0x00,0x00,0x00,0x00));
}

void hr_gem(int cx, int cy, int r, int ih)
{
	static const u64 neon    = GS_SETREG_RGBAQ(0x00,0xf0,0xff,0x00,0x00);
	static const u64 magenta = GS_SETREG_RGBAQ(0xff,0x00,0xd0,0x00,0x00);
	float fx = (float)cx, fy = (float)cy, fr = (float)r, fi = (float)ih;
	float pts[16];

	pts[0] = fx;       pts[1] = fy - fr;
	pts[2] = fx + fr;  pts[3] = fy;
	pts[4] = fx;       pts[5] = fy + fr;
	pts[6] = fx - fr;  pts[7] = fy;
	pts[8] = fx + fi;  pts[9]  = fy - fi;
	pts[10] = fx + fi; pts[11] = fy + fi;
	pts[12] = fx - fi; pts[13] = fy + fi;
	pts[14] = fx - fi; pts[15] = fy - fi;
	gsKit_prim_triangle_fan(gs, pts, 4, Z_GEM_A, neon);
	gsKit_prim_triangle_fan(gs, pts + 8, 4, Z_GEM_B, magenta);
}

/* Color transport is a separate register-only call: a 5th stack arg
 * from LLVM codegen was lost in transport on PCSX2 2.8.2 (the store
 * lands in the jal delay slot). Keep every hr_ call at <= 4 args. */
void hr_set_color(u32 rgba)
{
	spriteRgba = rgba;
}

void hr_sprite(int x, int y, int w, int h)
{
	gsKit_prim_sprite(gs, (float)x, (float)y,
			(float)(x + w), (float)(y + h), Z_SPRITE, (u64)spriteRgba);
}

void hr_frame_end(void)
{
	gsKit_queue_exec(gs);
	gsKit_sync_flip(gs);
}

int main(void)
{
	SifInitRpc(0);

	gs = gsKit_init_global();

	dmaKit_init(D_CTRL_RELE_OFF, D_CTRL_MFD_OFF, D_CTRL_STS_UNSPEC,
		    D_CTRL_STD_OFF, D_CTRL_RCYC_8, 1 << DMA_CHANNEL_GIF);
	dmaKit_chan_init(DMA_CHANNEL_GIF);

	gsKit_init_screen(gs);
	gs->PrimAlphaEnable = GS_SETTING_ON;

	/* ps2sdk's padInit waits forever for the padman RPC server; the IOP
	 * modules must be loaded first (rom0 versions are always present). */
	SifLoadModule("rom0:SIO2MAN", 0, NULL);
	SifLoadModule("rom0:PADMAN", 0, NULL);

	padInit(0);
	padPortOpen(0, 0, padBuf);

	rust_main((u32)gs->Width, (u32)gs->Height);

	padPortClose(0, 0);
	padEnd();
	return 0;
}
