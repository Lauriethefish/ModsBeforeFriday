import "./css/AnimatedBackground.css";
import { useCallback, useEffect, useRef, useState } from 'react';

const BLOCK_FALL_SPEED = 0.025;
const BLOCK_ROTATION_SPEED = 0.025;
const BLOCK_SCALE_RANGE = 0.5;
const BLOCK_SCALE_SPEED = 0.01;

export class FallingBlockParticle {
	type:number = 0;

	start_position:[number,number] = [0,0];
	start_rotation:number = 0;
	start_scale:number = 1;
	start_brightness:number = 1;

	velocity:[number,number] = [0,0];
	angular_velocity:number = 0;
	scale_change_speed:number = 0;
	brightness_change_speed:number = 0;

	progress:number = 0;
	node:SVGElement;
	current_animation:Animation|null = null;

	constructor(svg:SVGSVGElement){
		this.node = createSvgNode("g");
		svg.appendChild(this.node);
		this.randomise_state(false);
	}

	randomise_state(start_at_top:boolean = true){
		this.type = Math.floor(2*Math.random());
		this.angular_velocity = (2*Math.random()-1)*BLOCK_ROTATION_SPEED;

		this.start_scale = 1-(2*Math.random()-1)*BLOCK_SCALE_RANGE;

		let start_x_percentage = Math.random();
		this.start_position = [
			start_x_percentage*(window.innerWidth+200*this.start_scale)-this.start_scale*100,
			Math.random()*(window.innerHeight+150*this.start_scale)-this.start_scale*100
		];

		if(start_at_top){
			this.start_position[1] = -100*this.start_scale;
		}

		let drop_angle = Math.random() * Math.PI/2+Math.PI/4;	//	Calculate the angle at which the block will be moving

		this.velocity = [BLOCK_FALL_SPEED*Math.cos(drop_angle), BLOCK_FALL_SPEED*Math.sin(drop_angle)];	//	Calculate the velocity of the block


		this.progress = 0;
		this.update_node();
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
}

export function AnimatedBackground(){
	let svg:SVGSVGElement = createSvgNode("svg", {
		id:"anim-bg",
		viewBox:`0 0 ${window.innerWidth} ${window.innerHeight}`
	}) as SVGSVGElement;

	document.body.appendChild(svg);

	window.addEventListener("resize", ()=>{
		svg.setAttribute("viewBox", `0 0 ${window.innerWidth} ${window.innerHeight}`);
	});

	let particles = [];

	for(let i=0; i<100; i++){
		particles.push(new FallingBlockParticle(svg));

		//svg.appendChild(AnimatedBlock());
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