export default function Loader() {
	return (
		<>
		<style jsx>{`
			@keyframes spin {
				100% {
					transform: rotate(1turn);
				}
			}
			
			.loader {
				width: 50px;
				aspect-ratio: 1;
				border-radius: 50%;
				border: 8px solid transparent;
				border-right-color: #ffa50097;
				position: relative;
				animation: spin 1s infinite linear;
			}
			
			.loader::before,
			.loader::after {
				content: "";
				position: absolute;
				inset: -8px;
				border-radius: 50%;
				border: 8px solid transparent;
				border-right-color: #ffa50097;
				animation: spin 2s infinite linear;
			}
			
			.loader::after {
				animation-duration: 4s;
			}
		`}</style>
		<div className="loader" />
		</>
	);
}