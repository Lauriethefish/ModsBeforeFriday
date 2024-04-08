import "../css/AnimatedBackground.css";
import { useRef } from 'react';

export function AnimatedBackground(){
	let thing = [];
	for(let i=0;i<100;i++){
		thing.push(i);
	}
	let bg = <svg id="anim-bg" viewBox="0 0 1000 1000" xmlns="http://www.w3.org/2000/svg">
		{
			thing.map((i)=>{
				return <AnimatedBlock key={i} index={i}/>
			})
		}
	</svg>;
	return bg;
}
export function AnimatedBlock({index}:{index:number}){
	const bgref = useRef(null);
	let block = <g className={(Math.random()>0.5)?"block red-block hidden green-block":"block hidden green-block"} ref={bgref}>
					<rect x="-50" y="-50" width="100" height="100" rx="20" ry="20"/>
					<path d="M -40 -40 L 40 -40 L 40 -30 L 0 -20 L -40 -30 Z"/>
				</g>;
	
	function generateNewAnimation(){
		let elem:any = bgref.current;
		if(!elem) return;
		if(Math.random()>0.5) elem.classList.toggle("red-block");

		const bg = document.getElementById("anim-bg");
		if(!bg) return;

		let start = [Math.random()*(document.body.clientWidth+200)-bg.clientLeft,-100];
		let angle = Math.random() * Math.PI/2+Math.PI/4;
		const speed = 0.05;//0.2
		let vel = [speed*Math.cos(angle), speed*Math.sin(angle)];

		let time = 1400/vel[1];
		let end = [vel[0] * time + start[0], vel[1] * time + start[1]];
		
		if(end[0] > 0 && time > -100){

		}
		let filters = `brightness(${1+0.6*(Math.random()-0.5)})`;

		let animation = elem.animate([
			{ transform: `translate(${start[0]}px, ${start[1]}px) rotate(${(Math.random()-0.5)*Math.PI*4}rad) scale(${1.5-Math.random()})`, filter: filters },
			{ transform: `translate(${end[0]}px, ${end[1]}px) rotate(${(Math.random()-0.5)*Math.PI*4}rad) scale(${1.5-Math.random()})`, filters: filters }],
			{duration: time, iterations: 1});
		animation.onfinish = generateNewAnimation;
	}
	setTimeout(()=>{
		let elem:any = bgref.current;
		elem.classList.remove("hidden");
		elem.classList.remove("green-block");
		generateNewAnimation();
	},60000*Math.random());
	
	return block;
}