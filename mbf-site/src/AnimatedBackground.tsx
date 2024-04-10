import "./css/AnimatedBackground.css";
import { useRef } from 'react';

const BLOCK_SPEED = 0.025;



export function AnimatedBackground(body: HTMLBodyElement){
	let svg:SVGSVGElement = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("id", "anim-bg");
    svg.setAttribute("viewBox", `0 0 ${window.innerWidth} ${window.innerHeight}`);

	body.appendChild(svg);

	window.addEventListener("resize", ()=>{
		svg.setAttribute("viewBox", `0 0 ${window.innerWidth} ${window.innerHeight}`);
	});

	for(let i=0;i<200;i++){
		svg.appendChild(AnimatedBlock());
	}
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

		let vel = [BLOCK_SPEED*Math.cos(angle), BLOCK_SPEED*Math.sin(angle)];	//	Calculate the velocity of the block

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