export default class ShakeGesture {
    reset() {
        this.sample = null;
        this.direction = null;
        this.turns = [];
        this.fast = false;
    }

    constructor() {
        this.reset();
    }

    push(x, y, now) {
        this.fast = false;
        const previous = this.sample;
        if (!previous || now - previous.time > 150) {
            this.reset();
            this.sample = {x, y, time: now};
            return false;
        }

        const dx = x - previous.x;
        const dy = y - previous.y;
        const length = Math.hypot(dx, dy);
        if (length < 24)
            return false;

        this.sample = {x, y, time: now};
        this.fast = length >= (now - previous.time) * 0.55;
        if (!this.fast) {
            this.direction = null;
            this.turns = [];
            return false;
        }
        const direction = {x: dx / length, y: dy / length};
        const previousDirection = this.direction;
        this.direction = direction;
        if (!previousDirection || direction.x * previousDirection.x + direction.y * previousDirection.y > -0.65)
            return false;

        this.turns = this.turns.filter(time => now - time <= 420);
        this.turns.push(now);
        if (this.turns.length < 3)
            return false;
        this.turns = [];
        return true;
    }
}
