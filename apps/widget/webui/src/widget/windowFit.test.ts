import { beforeEach, describe, expect, it, vi } from 'vitest';

const ipc = vi.hoisted(() => ({ layoutWidget: vi.fn() }));
vi.mock('../lib/ipc', () => ({ commands: ipc }));

import { createLayout, sameRegions, type Box } from './windowFit';

const pill: Box = { x: 210, y: 280, width: 172, height: 40 };
const recording = { size: { width: 172, height: 40 }, inset: 8 };
const idle = { size: { width: 44, height: 44 }, inset: 0 };

describe('createLayout', () => {
  beforeEach(() => {
    ipc.layoutWidget.mockReset();
    ipc.layoutWidget.mockResolvedValue(pill);
  });

  it('asks where the pill goes and draws it there, without resizing anything', async () => {
    const show = vi.fn();
    await createLayout(show)(recording);
    expect(ipc.layoutWidget).toHaveBeenCalledWith(recording.size, 8);
    expect(show).toHaveBeenCalledWith(pill);
  });

  it('does nothing again for the target it already laid out', async () => {
    const layout = createLayout(vi.fn());
    await layout(recording);
    await layout({ ...recording, size: { ...recording.size } });
    expect(ipc.layoutWidget).toHaveBeenCalledTimes(1);
  });

  it('skips a target that a newer one replaced before its turn', async () => {
    const show = vi.fn();
    const layout = createLayout(show);
    void layout(recording);
    await layout(idle);
    expect(ipc.layoutWidget.mock.calls.map((call) => call[0])).toEqual([idle.size]);
    expect(show).toHaveBeenCalledTimes(1);
  });
});

describe('sameRegions', () => {
  it('compares hit regions point for point', () => {
    expect(sameRegions([pill], [{ ...pill }])).toBe(true);
    expect(sameRegions([pill], [{ ...pill, width: 171 }])).toBe(false);
    expect(sameRegions([], [pill])).toBe(false);
  });
});
