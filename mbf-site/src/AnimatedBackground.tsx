import "./css/AnimatedBackground.css";

//	Speed (in pixels per millisecond) at which the blocks will move
const BLOCK_FALL_SPEED = 10 /1000;

//	Maximum speed at which the blocks will rotate (in radians per millisecond). Speed and direction are randomised per block.
const BLOCK_ROTATION_SPEED = 0.2 /1000;

//	How big/small the blocks should be on average. The value of 0.4 means that the blocks will be scaled by a value between 0.6 and 1.4.
//	The exact value is randomised, and the actual speed at which the scale changes is calculated based on the value provided here.
const BLOCK_SCALE_RANGE = 0.4;
//	Same as above but for brightness instead of size.
const BLOCK_BRIGHTNESS_RANGE = 0.4;

//	The above values are used to estimate how quickly the brightness and scale will change, but if the window is resized the blocks which have already been spawned cannot have these speeds recalculated, and will keep using them until they go offscreen.
//	This value determines how far the brightness and scale can go from the originally predicted values before being clamped.
const BLOCK_VALUE_SAFETY_LIMIT = 0.1;

//	How often on average each block should have its animation recalculated (in milliseconds). The actual value will be randomised to prevent blocks from combining into large batches.
const BLOCK_ANIMATION_AVERAGE_LIFETIME = 5000;
//	How far from the average animation lifetime the actual animation lifetime can be (in milliseconds)
const BLOCK_ANIMATION_LIFETIME_RANGE = 1000;

//	Amount of blocks per pixel. If the amount of blocks on screen differs too much blocks will be added/removed.
const BLOCK_DENSITY = 0.00008;

export class FallingBlockParticle {
	type:number = 0;

	position:[number,number] = [0,0];
	rotation:number = 0;
	scale:number = 1;
	brightness:number = 1;

	velocity:[number,number] = [0,0];
	angular_velocity:number = 0;
	scale_change_speed:number = 0;
	brightness_change_speed:number = 0;

	node:SVGElement;
	animation:Animation|null = null;
	static onExit:Function|null = null;

	constructor(svg:SVGSVGElement, start_at_top:boolean=false){
		this.node = createSvgNode("g");
		svg.appendChild(this.node);
		this.randomise_state(start_at_top);
	}

	randomise_state(start_at_top:boolean = true){
		this.type = Math.floor(7*Math.random())-2;
		if(this.type<0) this.type+=2;
		this.angular_velocity = (2*Math.random()-1)*BLOCK_ROTATION_SPEED;

		this.rotation = 1-(2*Math.random()-1)*Math.PI;

		let start = 2*Math.random()-1;
		let end = 2*Math.random()-1;

		this.scale = 1-start*BLOCK_SCALE_RANGE;
		let end_scale = 1-end*BLOCK_SCALE_RANGE;
		this.brightness = 1-start*BLOCK_BRIGHTNESS_RANGE;
		let end_brightness = 1-end*BLOCK_BRIGHTNESS_RANGE;

		let start_x_percentage = Math.random();	//	Figure out where the block should spawn
		this.position = [
			start_x_percentage*(window.innerWidth+200*this.scale)-this.scale*100,
			Math.random()*(window.innerHeight+150*this.scale)-this.scale*100
		];

		if(start_at_top){
			this.position[1] = -100*this.scale;
		}

		let drop_angle = Math.random() * Math.PI/2+Math.PI/4;	//	Calculate the angle at which the block will be moving

		this.velocity = [BLOCK_FALL_SPEED*Math.cos(drop_angle), BLOCK_FALL_SPEED*Math.sin(drop_angle)];	//	Calculate the velocity of the block

		//	Estimate the time needed for the block to reach the bottom of the screen, and calculate how quickly the block should get brighter/bigger based on that estimate.
		let time_est = (window.innerHeight - this.position[1] + end_scale)/this.velocity[1];
		this.scale_change_speed = (end_scale-this.scale)/time_est;

		if(this.type!==4)this.brightness_change_speed = (this.brightness-end_brightness)/time_est;


		this.animation = null;
		this.update_node();
		this.update_progress(0);
	}

	update_node(){
		while(this.node.lastChild)this.node.removeChild(this.node.lastChild);
		this.node.classList.remove("block");
		this.node.classList.remove("red-block");
		this.node.classList.remove("bomb");

		switch(this.type){
			case 0:
				this.node.classList.add("block");
				this.node.appendChild(createSvgNode("rect", {
					x:-50,
					y:-50,
					width: 100,
					height: 100,
					rx: 20,
					ry: 20,
				}));
				this.node.appendChild(createSvgNode("path", {
					d: "M -40 -40 L 40 -40 L 40 -30 L 0 -10 L -40 -30 Z"
				}));
				break;
			case 2:
				this.node.classList.add("block");
				this.node.appendChild(createSvgNode("rect", {
					x:-50,
					y:-50,
					width: 100,
					height: 100,
					rx: 20,
					ry: 20,
				}));
				this.node.appendChild(createSvgNode("ellipse", {
					cx:0,
					cy:0,
					rx:20,
					ry:20
				}));
				break;
			case 1:
				this.node.classList.add("block");
				this.node.classList.add("red-block");
				this.node.appendChild(createSvgNode("rect", {
					x:-50,
					y:-50,
					width: 100,
					height: 100,
					rx: 20,
					ry: 20,
				}));
				this.node.appendChild(createSvgNode("path", {
					d: "M -40 -40 L 40 -40 L 40 -30 L 0 -10 L -40 -30 Z"
				}));
				break;
			case 3:
				this.node.classList.add("block");
				this.node.classList.add("red-block");
				this.node.appendChild(createSvgNode("rect", {
					x:-50,
					y:-50,
					width: 100,
					height: 100,
					rx: 20,
					ry: 20,
				}));
				this.node.appendChild(createSvgNode("ellipse", {
					cx:0,
					cy:0,
					rx:20,
					ry:20
				}));
				break;
			case 4:
				this.node.classList.add("bomb");
				this.node.appendChild(createSvgNode("path", {
					d:"M 16.588 25.261 L 0.271 25.261 L -9.292 58.594 L -8.873 25.26 L -19.645 25.26 L -24.671 29.566 L -21.536 21.708 L -25.928 6.658 L -66.658 9.545 L -28.483 -2.096 L -31.32 -11.818 L -30.336 -12.56 L -41.991 -20.297 L -24.148 -17.223 L -17.527 -22.213 L -32.214 -56.515 L -9.796 -28.04 L -5.567 -31.228 L -2.62 -47.336 L 0.907 -33.318 L 13.827 -23.318 L 46.439 -48.293 L 21.605 -17.299 L 26.002 -13.896 L 39.033 -14.184 L 27.412 -7.903 L 24.15 2.09 L 60.606 22.849 L 21.106 11.417 L 18.779 18.547 L 25.405 33.345 L 16.652 25.066 L 16.588 25.261 Z",

				}));
				break;

		}
	}
	update_progress(delta_time:number){
		if(delta_time > 0){
			let state = this.calculate_next_state(delta_time);
			this.position = state.position;
			this.rotation = state.rotation;
			this.scale = state.scale;
			this.brightness = state.brightness;
		}
		if((this.position[1] > 100*this.scale + window.innerHeight) || (this.position[0] < -100*this.scale) || (this.position[0] > 100*this.scale + window.innerWidth)){
			if(FallingBlockParticle.onExit) {
				FallingBlockParticle.onExit(this);
			}else{
				this.randomise_state();
			}
		}else{
			this.update_animation();
		}
	}
	calculate_next_state(delta_time:number){
		let state:{position:[number,number],rotation:number,scale:number,brightness:number} = {
			position:[
				this.position[0] + this.velocity[0] * delta_time,
				this.position[1] + this.velocity[1] * delta_time
			],
			rotation: this.rotation + this.angular_velocity * delta_time,
			scale: Math.min(Math.max(this.scale + this.scale_change_speed * delta_time, 1-BLOCK_VALUE_SAFETY_LIMIT-BLOCK_SCALE_RANGE), 1+BLOCK_VALUE_SAFETY_LIMIT+BLOCK_SCALE_RANGE),
			brightness: Math.min(Math.max(this.brightness + this.brightness_change_speed * delta_time, 1-BLOCK_VALUE_SAFETY_LIMIT-BLOCK_BRIGHTNESS_RANGE), 1+BLOCK_VALUE_SAFETY_LIMIT+BLOCK_BRIGHTNESS_RANGE)
		};
		return state;
	}
	update_animation(){
		let pass_time = BLOCK_ANIMATION_AVERAGE_LIFETIME + (2*Math.random()-1)*BLOCK_ANIMATION_LIFETIME_RANGE;

		let next_state = this.calculate_next_state(pass_time);
		let keyframes = [
			{ transform: `translate(${this.position[0]}px, ${this.position[1]}px) rotate(${this.rotation}rad) scale(${this.scale})`, filter: `brightness(${this.brightness})` },
			{ transform: `translate(${next_state.position[0]}px, ${next_state.position[1]}px) rotate(${next_state.rotation}rad) scale(${next_state.scale})`, filter: `brightness(${next_state.brightness})` }
		];
		this.animation = this.node.animate(keyframes, pass_time);
		this.animation.onfinish = ()=>{
			this.update_progress(this.animation?.currentTime as number);
		}
		this.animation.onremove
	}
}

export function AnimatedBackground() {
	let svg = createSvgNode("svg", {
		id:"anim-bg",
		viewBox:`0 0 ${window.innerWidth} ${window.innerHeight}`
	}) as SVGSVGElement;
	let defs = createSvgNode("defs", {});
	let gradient = createSvgNode("radialGradient", {id:"bomb-gradient"});
	gradient.appendChild(createSvgNode("stop", { offset:0, style:"stop-color: rgb(20, 20, 20);"}));
	gradient.appendChild(createSvgNode("stop", { offset:0.75, style:"stop-color: rgb(3, 3, 3);"}));
	defs.appendChild(gradient);
	svg.appendChild(defs);

	document.body.appendChild(svg);

	window.addEventListener("resize", ()=>{
		svg.setAttribute("viewBox", `0 0 ${window.innerWidth} ${window.innerHeight}`);
	});

	let particles: FallingBlockParticle[] = [];

	for(let i=0; i<calculateBlockCount(); i++){
		particles.push(new FallingBlockParticle(svg));
	}
	setInterval(()=>{
		if(particles.length < calculateBlockCount()){
			FallingBlockParticle.onExit = null;
			particles.push(new FallingBlockParticle(svg, true));
			return;
		}
		if(particles.length > Math.ceil(calculateBlockCount())){
			FallingBlockParticle.onExit = (p:FallingBlockParticle)=>{
				if(particles.length > Math.ceil(calculateBlockCount())){
					particles.splice(particles.findIndex((e)=>(e===p)), 1);
				}else{
					FallingBlockParticle.onExit = null;
				}
			};
		}
	}, 500);
}
export function calculateBlockCount(){
	return Math.max(window.innerWidth*window.innerHeight*BLOCK_DENSITY/window.devicePixelRatio, 20);
}

function createSvgNode(tag:string, attributes:any = {}){
	let svg:SVGElement = document.createElementNS("http://www.w3.org/2000/svg", tag);
	for(let attr in attributes){
		if(attr === "className") attr = "class";
		svg.setAttribute(attr, attributes[attr]);
	}
	return svg;
}
