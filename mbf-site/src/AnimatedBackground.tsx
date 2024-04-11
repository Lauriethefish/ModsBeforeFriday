import "./css/AnimatedBackground.css";
import { useCallback, useEffect, useRef, useState } from 'react';

const BLOCK_FALL_SPEED = 10/1000;
const BLOCK_ROTATION_SPEED = 0.2/1000;
const BLOCK_SCALE_RANGE = 0.4;//0.3;
const BLOCK_BRIGHTNESS_RANGE = 0.4;//0.30;
const BLOCK_VALUE_SAFETY_LIMIT = 0.1;
const BLOCK_ANIMATION_AVERAGE_LIFETIME = 5000;
const BLOCK_ANIMATION_LIFETIME_RANGE = 1000;

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

	constructor(svg:SVGSVGElement){
		this.node = createSvgNode("g");
		svg.appendChild(this.node);
		this.randomise_state(false);
	}

	randomise_state(start_at_top:boolean = true){
		this.type = Math.floor(2*Math.random());
		this.angular_velocity = (2*Math.random()-1)*BLOCK_ROTATION_SPEED;

		this.rotation = 1-(2*Math.random()-1)*Math.PI;

		let start = 2*Math.random()-1;	//	Make small blocks more likely to grow, and big blocks more likely to shrink.
		let end = 2*Math.random()-1;	//	Make small blocks more likely to grow, and big blocks more likely to shrink.

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

		let time_est = (window.innerHeight - this.position[1] + end_scale)/this.velocity[1];
		this.scale_change_speed = (end_scale-this.scale)/time_est;

		//limiter = 2*Math.random()-1;	//	Make bright blocks more likely to darken, and dark blocks more likely to lighten up.
		//offset = Math.random();
		this.brightness_change_speed = (this.brightness-end_brightness)/time_est;
		//this.brightness_change_speed = ((2-Math.abs(limiter))*offset-1+limiter) * BLOCK_BRIGHTNESS_SPEED;


		this.animation = null;
		this.update_node();
		this.update_progress(0);
	}

	update_node(){
		while(this.node.lastChild)this.node.removeChild(this.node.lastChild);
		this.node.classList.forEach((c)=>this.node.classList.remove(c));

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
					d: "M -40 -40 L 40 -40 L 40 -30 L 0 -20 L -40 -30 Z"
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
					d: "M -40 -40 L 40 -40 L 40 -30 L 0 -20 L -40 -30 Z"
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
			this.randomise_state();
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

export function AnimatedBackground(){
	let svg = createSvgNode("svg", {
		id:"anim-bg",
		viewBox:`0 0 ${window.innerWidth} ${window.innerHeight}`
	}) as SVGSVGElement;
	let defs = createSvgNode("defs", {});
	let gradient = createSvgNode("radialGradient", {id:"bomb-gradient"});
	gradient.appendChild(createSvgNode("stop", { offset:0.05, style:"stop-color: rgb(57, 57, 57);"}));
	gradient.appendChild(createSvgNode("stop", { offset:0.75, style:"stop-color: rgb(14, 14, 14);"}));
	defs.appendChild(gradient);
	svg.appendChild(defs);

	document.body.appendChild(svg);

	window.addEventListener("resize", ()=>{
		svg.setAttribute("viewBox", `0 0 ${window.innerWidth} ${window.innerHeight}`);
	});

	let particles = [];

	for(let i=0; i<100; i++){
		particles.push(new FallingBlockParticle(svg));
	}
}

function createSvgNode(tag:string, attributes:any = {}){
	let svg:SVGElement = document.createElementNS("http://www.w3.org/2000/svg", tag);
	for(let attr in attributes){
		if(attr === "className") attr = "class";
		svg.setAttribute(attr, attributes[attr]);
	}
	return svg;
}

export function AnimatedBlock(){
	let block = document.createElementNS("http://www.w3.org/2000/svg", "g");
	{
		block.classList.add("block");
		block.classList.add("hidden");
		if(Math.random()>0.5) block.classList.add("red-block");

		let rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
		rect.setAttribute("x", "-50");
		rect.setAttribute("y", "-50");
		rect.setAttribute("width", "100");
		rect.setAttribute("height", "100");
		rect.setAttribute("rx", "20");
		rect.setAttribute("ry", "20");
		block.appendChild(rect);

		let path = document.createElementNS("http://www.w3.org/2000/svg", "path");
		path.setAttribute("d", "M -40 -40 L 40 -40 L 40 -30 L 0 -20 L -40 -30 Z");
		block.appendChild(path);
	}
	/*<g className={(Math.random()>0.5)?"block red-block hidden green-block":"block hidden green-block"} ref={bgref}>
					<rect x="-50" y="-50" width="100" height="100" rx="20" ry="20"/>
					<path d="M -40 -40 L 40 -40 L 40 -30 L 0 -20 L -40 -30 Z"/>
				</g>;*/
	
	function generateNewAnimation(startPos:[number,number] | null = null){
		if(Math.random()>0.5) block.classList.toggle("red-block");

		const bg = document.getElementById("anim-bg");
		if(!bg) return;

		let startScale = 1.5-Math.random();
		let endScale = 1.5-Math.random();
		let maxSize = Math.max(startScale, endScale) * 100;
		
		if(!startPos){
			startPos = [Math.random()*(document.body.clientWidth+200)-maxSize,-maxSize];
		}

		let angle = Math.random() * Math.PI/2+Math.PI/4;	//	Calculate the angle at which the block will be moving

		let vel = [BLOCK_FALL_SPEED*Math.cos(angle), BLOCK_FALL_SPEED*Math.sin(angle)];	//	Calculate the velocity of the block

		let time = (window.innerHeight - startPos[1] + 2*maxSize)/vel[1];	//	Calculate how long the block will take to fall to the bottom of the screen

		let endPos:[number,number] = [vel[0] * time + startPos[0], vel[1] * time + startPos[1]];
		
		let filters = `brightness(${1+0.6*(Math.random()-0.5)})`;

		let keyframes = [
			{ transform: `translate(${startPos[0]}px, ${startPos[1]}px) rotate(${(Math.random()-0.5)*Math.PI*4}rad) scale(${startScale})`, filter: filters },
			{ transform: `translate(${endPos[0]}px, ${endPos[1]}px) rotate(${(Math.random()-0.5)*Math.PI*4}rad) scale(${endScale})`, filters: filters }
		];
		let animation = block.animate(keyframes, {duration: time, iterations: 1});

		animation.onfinish = ()=>{
			generateNewAnimation();
		};
	}
	setTimeout(()=>{
		//let elem:any = bgref.current;
		block.classList.remove("hidden");
		generateNewAnimation();
	}, 60000*Math.random());
	
	return block;
}